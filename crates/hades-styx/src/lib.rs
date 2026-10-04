//! # Styx - Simulador de Degradação de Rede e Perfis de Conexão
//!
//! SPEC-0012: Motor de testes de estresse e fidelidade de transporte QUIC/UDP
//! calibrado com distâncias e topologias físicas reais do Norte/Nordeste do Brasil.

use rand::Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::Notify;

/// Perfis de degradação e características físicas de enlace (SPEC-0012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyxProfile {
    /// Desativado (passthrough direto sem latência artificial).
    Disabled,
    /// Fibra Óptica Norte/Nordeste -> SP (48 ms RTT, ±3 ms jitter, 0.05% perda).
    Fiber,
    /// Banda Larga Urbana / Coaxial (65 ms RTT, ±8 ms jitter, 0.2% perda).
    City,
    /// Celular 4G/5G em Capitais (95 ms RTT, ±28 ms jitter, 1.5% perda).
    Mobile,
    /// 3G/4G Rural e Interiores ribeirinhos (220 ms RTT, ±85 ms jitter, 7.0% perda).
    RuralMobile,
    /// Satélite LEO / Órbita Baixa (68 ms RTT, ±20 ms jitter, 1.8% perda).
    LowOrbit,
    /// Satélite Geoestacionário GEO Banda Ka/Ku (680 ms RTT, ±65 ms jitter, 4.0% perda).
    Geostationary,
    /// Apocalipse / Condições Extremamente Hostis (1750 ms RTT [1500~2000 ms], ±250 ms jitter, 20.0% perda).
    Apocalypse,
}

impl StyxProfile {
    /// RTT médio bidirecional em milissegundos.
    pub const fn rtt_ms(&self) -> f32 {
        match self {
            Self::Disabled => 0.0,
            Self::Fiber => 48.0,
            Self::City => 65.0,
            Self::Mobile => 95.0,
            Self::RuralMobile => 220.0,
            Self::LowOrbit => 68.0,
            Self::Geostationary => 680.0,
            Self::Apocalypse => 1750.0,
        }
    }

    /// Variação estocástica de atraso (Jitter ±ms).
    pub const fn jitter_ms(&self) -> f32 {
        match self {
            Self::Disabled => 0.0,
            Self::Fiber => 3.0,
            Self::City => 8.0,
            Self::Mobile => 28.0,
            Self::RuralMobile => 85.0,
            Self::LowOrbit => 20.0,
            Self::Geostationary => 65.0,
            Self::Apocalypse => 250.0,
        }
    }

    /// Taxa de descarte de pacotes (0.0 = sem perda, 0.01 = 1%).
    pub const fn loss_rate(&self) -> f64 {
        match self {
            Self::Disabled => 0.0,
            Self::Fiber => 0.0005,
            Self::City => 0.002,
            Self::Mobile => 0.015,
            Self::RuralMobile => 0.070,
            Self::LowOrbit => 0.018,
            Self::Geostationary => 0.040,
            Self::Apocalypse => 0.200,
        }
    }

    /// Nome amigável do perfil.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Disabled => "Desligado",
            Self::Fiber => "Fibra Óptica (Norte/Nordeste -> SP)",
            Self::City => "Banda Larga Cidade",
            Self::Mobile => "Rede Móvel 4G/5G",
            Self::RuralMobile => "Móvel Rural / Interior",
            Self::LowOrbit => "Satélite Órbita Baixa (LEO)",
            Self::Geostationary => "Satélite Geoestacionário (GEO)",
            Self::Apocalypse => "Apocalipse (Lag Extremo 1500~2000ms, 20% perda)",
        }
    }

    /// Identificador curto de telemetria / HUD.
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::Disabled => "DESLIGADO",
            Self::Fiber => "FIBRA 48ms ±3ms",
            Self::City => "CIDADE 65ms ±8ms",
            Self::Mobile => "MOVEL 95ms ±28ms",
            Self::RuralMobile => "RURAL 220ms ±85ms",
            Self::LowOrbit => "LEO 68ms ±20ms",
            Self::Geostationary => "GEO 680ms ±65ms",
            Self::Apocalypse => "APOCALIPSE 1750ms ±250ms (20% loss)",
        }
    }

    /// Converte texto amigável em perfil correspondente.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            "disabled" | "off" | "desligado" | "false" | "0" | "none" => Some(Self::Disabled),
            "fibra" | "fiber" | "ftth" => Some(Self::Fiber),
            "cidade" | "city" | "urbano" | "coaxial" => Some(Self::City),
            "movel" | "móvel" | "mobile" | "4g" | "5g" | "celular" => Some(Self::Mobile),
            "rural" | "rural móvel" | "rural movel" | "3g" => Some(Self::RuralMobile),
            "low_orbit" | "loworbit" | "low orbit" | "leo" | "orbita baixa" | "órbita baixa" => {
                Some(Self::LowOrbit)
            }
            "estacionario" | "estacionário" | "geo" | "geostationary" | "satelite" | "satélite" => {
                Some(Self::Geostationary)
            }
            "apocalipse" | "apocalypse" | "caos" | "chaos" | "inferno" | "doom" => {
                Some(Self::Apocalypse)
            }
            _ => None,
        }
    }

    /// Amostra a latência unilateral de viagem (*one-way delay* = RTT/2 + Jitter).
    pub fn sample_one_way_delay(&self) -> Duration {
        if *self == Self::Disabled {
            return Duration::ZERO;
        }

        let base_half = self.rtt_ms() / 2.0;
        let mut rng = rand::thread_rng();
        let jitter_factor: f32 = rng.gen_range(-1.0..=1.0);
        let delay_ms = (base_half + jitter_factor * self.jitter_ms()).max(0.0);

        Duration::from_secs_f32(delay_ms / 1000.0)
    }

    /// Avalia se o pacote corrente deve ser descartado por perda estocástica.
    pub fn should_drop(&self) -> bool {
        let rate = self.loss_rate();
        if rate <= 0.0 {
            false
        } else {
            let mut rng = rand::thread_rng();
            rng.gen::<f64>() < rate
        }
    }
}

/// Configuração do interceptor Styx.
#[derive(Debug, Clone)]
pub struct StyxConfig {
    pub profile: StyxProfile,
}

impl Default for StyxConfig {
    fn default() -> Self {
        // Default desligado em prod; se STYX_PROFILE estiver no ambiente, carrega-o
        let profile = std::env::var("STYX_PROFILE")
            .ok()
            .and_then(|val| StyxProfile::from_str_loose(&val))
            .unwrap_or(StyxProfile::Disabled);

        Self { profile }
    }
}

impl StyxConfig {
    /// Analisa argumentos de linha de comando ou variáveis de ambiente para detectar `--styx`.
    /// Exemplos suportados:
    /// - `--styx` (adota `StyxProfile::Fiber` por padrão)
    /// - `--styx fibra`
    /// - `--styx movel`
    /// - `--styx rural`
    /// - `--styx cidade`
    /// - `--styx low_orbit`
    /// - `--styx estacionario`
    /// - `--styx apocalipse`
    /// - `-s <perfil>`
    pub fn from_args_and_env() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut profile = None;

        for (i, arg) in args.iter().enumerate() {
            if arg == "--styx" || arg == "-s" {
                if let Some(next_arg) = args.get(i + 1) {
                    if !next_arg.starts_with('-') {
                        if let Some(p) = StyxProfile::from_str_loose(next_arg) {
                            profile = Some(p);
                            break;
                        }
                    }
                }
                // Se `--styx` foi fornecido sem parâmetro subsequente, padrão é Fibra Norte/Nordeste
                if profile.is_none() {
                    profile = Some(StyxProfile::Fiber);
                    break;
                }
            } else if let Some(stripped) = arg.strip_prefix("--styx=") {
                if let Some(p) = StyxProfile::from_str_loose(stripped) {
                    profile = Some(p);
                    break;
                }
            }
        }

        if let Some(p) = profile {
            Self { profile: p }
        } else {
            Self::default()
        }
    }
}

/// Proxy transparente UDP para interceptação com latência simulada e descarte.
pub struct StyxProxy {
    listen_addr: SocketAddr,
    shutdown: Arc<Notify>,
}

impl StyxProxy {
    /// Inicia o proxy intermediário ouvindo em `127.0.0.1:0` e encaminhando para `target_addr`.
    pub async fn start(
        target_addr: SocketAddr,
        profile: StyxProfile,
    ) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind("127.0.0.1:0").await?;
        let listen_addr = socket.local_addr()?;
        let shutdown = Arc::new(Notify::new());

        let shutdown_rx = shutdown.clone();

        tokio::spawn(async move {
            let socket = Arc::new(socket);
            let mut buf = [0u8; 65535];
            let mut client_addr: Option<SocketAddr> = None;

            loop {
                tokio::select! {
                    _ = shutdown_rx.notified() => break,
                    res = socket.recv_from(&mut buf) => {
                        let (len, src) = match res {
                            Ok(pair) => pair,
                            Err(_) => break,
                        };

                        let packet_data = buf[..len].to_vec();

                        // Tráfego vindo do Cliente -> Encaminha para o Servidor Hades
                        if src != target_addr {
                            client_addr = Some(src);

                            if profile.should_drop() {
                                continue;
                            }

                            let delay = profile.sample_one_way_delay();
                            let sock = socket.clone();
                            tokio::spawn(async move {
                                if !delay.is_zero() {
                                    tokio::time::sleep(delay).await;
                                }
                                let _ = sock.send_to(&packet_data, target_addr).await;
                            });
                        }
                        // Tráfego vindo do Servidor Hades -> Encaminha de volta para o Cliente
                        else if let Some(cli) = client_addr {
                            if profile.should_drop() {
                                continue;
                            }

                            let delay = profile.sample_one_way_delay();
                            let sock = socket.clone();
                            tokio::spawn(async move {
                                if !delay.is_zero() {
                                    tokio::time::sleep(delay).await;
                                }
                                let _ = sock.send_to(&packet_data, cli).await;
                            });
                        }
                    }
                }
            }
        });

        Ok(Self {
            listen_addr,
            shutdown,
        })
    }

    /// Endereço UDP local onde o proxy está aguardando conexões.
    pub const fn local_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    /// Encerra o proxy.
    pub fn stop(&self) {
        self.shutdown.notify_waiters();
    }
}

impl Drop for StyxProxy {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styx_profile_from_str_loose() {
        assert_eq!(StyxProfile::from_str_loose("fibra"), Some(StyxProfile::Fiber));
        assert_eq!(StyxProfile::from_str_loose("fiber"), Some(StyxProfile::Fiber));
        assert_eq!(StyxProfile::from_str_loose("cidade"), Some(StyxProfile::City));
        assert_eq!(StyxProfile::from_str_loose("movel"), Some(StyxProfile::Mobile));
        assert_eq!(StyxProfile::from_str_loose("móvel"), Some(StyxProfile::Mobile));
        assert_eq!(StyxProfile::from_str_loose("rural"), Some(StyxProfile::RuralMobile));
        assert_eq!(StyxProfile::from_str_loose("low_orbit"), Some(StyxProfile::LowOrbit));
        assert_eq!(StyxProfile::from_str_loose("estacionario"), Some(StyxProfile::Geostationary));
        assert_eq!(StyxProfile::from_str_loose("apocalipse"), Some(StyxProfile::Apocalypse));
        assert_eq!(StyxProfile::from_str_loose("apocalypse"), Some(StyxProfile::Apocalypse));
        assert_eq!(StyxProfile::from_str_loose("caos"), Some(StyxProfile::Apocalypse));
        assert_eq!(StyxProfile::from_str_loose("inferno"), Some(StyxProfile::Apocalypse));
        assert_eq!(StyxProfile::from_str_loose("disabled"), Some(StyxProfile::Disabled));
        assert_eq!(StyxProfile::from_str_loose("desconhecido"), None);
    }

    #[test]
    fn test_styx_profile_delays_and_bounds() {
        let fiber = StyxProfile::Fiber;
        assert_eq!(fiber.rtt_ms(), 48.0);
        assert_eq!(fiber.jitter_ms(), 3.0);

        for _ in 0..100 {
            let delay = fiber.sample_one_way_delay();
            let ms = delay.as_secs_f32() * 1000.0;
            // Metade de 48ms é 24ms; com jitter ±3ms, fica entre 21ms e 27ms
            assert!((20.0..=28.0).contains(&ms), "Amostra fora dos limites esperados: {}ms", ms);
        }

        let apoc = StyxProfile::Apocalypse;
        assert_eq!(apoc.rtt_ms(), 1750.0);
        assert_eq!(apoc.jitter_ms(), 250.0);
        assert_eq!(apoc.loss_rate(), 0.20);
        for _ in 0..100 {
            let delay = apoc.sample_one_way_delay();
            let ms = delay.as_secs_f32() * 1000.0;
            // Metade de 1750ms é 875ms; com jitter ±250ms, fica entre 625ms e 1125ms (RTT total ~1500ms..2000ms)
            assert!((600.0..=1150.0).contains(&ms), "Amostra apocalipse fora dos limites: {}ms", ms);
        }
    }
}
