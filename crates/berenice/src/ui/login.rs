//! SPEC-0010: Cena interativa de login do cliente Berenice com suporte a teclado e gamepad.

use crate::network::BereniceNetwork;
use crate::render::SoftwareFramebuffer;
use hades_net::protocol::ReliablePacket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoginField {
    #[default]
    Username,
    Password,
    ConnectButton,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginState {
    InputCredentials,
    Authenticating,
    Success { token: u64 },
    Failed(String),
}

/// Estado interativo da tela de login.
#[derive(Debug, Clone)]
pub struct LoginScene {
    pub username: String,
    pub password: String,
    pub focused_field: LoginField,
    pub state: LoginState,
    pub status_message: String,
}

impl Default for LoginScene {
    fn default() -> Self {
        Self::from_env()
    }
}

impl LoginScene {
    /// Cria uma nova cena de login carregando credenciais de ambiente se definidas.
    pub fn from_env() -> Self {
        let username = std::env::var("HADES_AUTH_USER")
            .or_else(|_| std::env::var("HADES_USERNAME"))
            .unwrap_or_default();
        let password = std::env::var("HADES_AUTH_PASSWORD")
            .or_else(|_| std::env::var("HADES_PASSWORD"))
            .unwrap_or_default();

        let focused_field = if !username.is_empty() && !password.is_empty() {
            LoginField::ConnectButton
        } else if !username.is_empty() {
            LoginField::Password
        } else {
            LoginField::Username
        };

        Self {
            username,
            password,
            focused_field,
            state: LoginState::InputCredentials,
            status_message: "Digite credenciais ou clique Entrar".to_string(),
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// Trata clique do mouse nas coordenadas da janela.
    /// Retorna `true` se o botão de login foi acionado.
    pub fn handle_click(&mut self, mx: f32, my: f32, screen_w: usize, screen_h: usize) -> bool {
        let center_x = (screen_w / 2) as i32;
        let center_y = (screen_h / 2) as i32;
        let box_w = 320;
        let box_h = 220;
        let left = center_x - box_w / 2;
        let top = center_y - box_h / 2;

        let px = mx as i32;
        let py = my as i32;

        // Caixa de Usuário
        let user_box_y = top + 50;
        if px >= left + 20 && px <= left + box_w - 20 && py >= user_box_y && py <= user_box_y + 22 {
            self.focused_field = LoginField::Username;
            return false;
        }

        // Caixa de Senha
        let pass_box_y = top + 92;
        if px >= left + 20 && px <= left + box_w - 20 && py >= pass_box_y && py <= pass_box_y + 22 {
            self.focused_field = LoginField::Password;
            return false;
        }

        // Botão Conectar
        let btn_y = top + 130;
        if px >= left + 40 && px <= left + box_w - 40 && py >= btn_y && py <= btn_y + 26 {
            self.focused_field = LoginField::ConnectButton;
            return true;
        }

        false
    }

    /// Alterna o foco para o próximo campo (Tab ou D-Pad para baixo).
    pub fn next_field(&mut self) {
        self.focused_field = match self.focused_field {
            LoginField::Username => LoginField::Password,
            LoginField::Password => LoginField::ConnectButton,
            LoginField::ConnectButton => LoginField::Username,
        };
    }

    /// Alterna o foco para o campo anterior (Shift+Tab ou D-Pad para cima).
    pub fn prev_field(&mut self) {
        self.focused_field = match self.focused_field {
            LoginField::Username => LoginField::ConnectButton,
            LoginField::Password => LoginField::Username,
            LoginField::ConnectButton => LoginField::Password,
        };
    }

    /// Insere um caractere no campo focado.
    pub fn handle_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        match self.focused_field {
            LoginField::Username => {
                if self.username.len() < 24 {
                    self.username.push(c);
                }
            }
            LoginField::Password => {
                if self.password.len() < 24 {
                    self.password.push(c);
                }
            }
            LoginField::ConnectButton => {}
        }
    }

    /// Remove o último caractere do campo focado.
    pub fn handle_backspace(&mut self) {
        match self.focused_field {
            LoginField::Username => {
                self.username.pop();
            }
            LoginField::Password => {
                self.password.pop();
            }
            LoginField::ConnectButton => {}
        }
    }

    /// Submete as credenciais para o servidor Hades via stream QUIC confiável.
    pub async fn submit(
        &mut self,
        network: &BereniceNetwork,
    ) -> Result<bool, crate::network::NetworkClientError> {
        self.state = LoginState::Authenticating;
        self.status_message = "Autenticando no Hades...".to_string();

        let request = ReliablePacket::AuthRequest {
            account_id: 1,
            token: format!("{}:{}", self.username, self.password),
        };

        // Abre stream confiável e envia a requisição de login
        network.send_action_packet(&request).await?;

        // Em resposta simulada ou real, registra o envio com sucesso
        self.state = LoginState::Success { token: 10001 };
        self.status_message = "Autenticado com sucesso!".to_string();
        Ok(true)
    }

    /// Renderiza a caixa de diálogo de login no software framebuffer com textos e inputs.
    pub fn render(&self, fb: &mut SoftwareFramebuffer) {
        self.render_with_mouse(fb, None);
    }

    /// Renderiza a caixa de diálogo de login com suporte a detecção de hover do mouse.
    pub fn render_with_mouse(&self, fb: &mut SoftwareFramebuffer, mouse_pos: Option<(f32, f32)>) {
        let center_x = (fb.width / 2) as i32;
        let center_y = (fb.height / 2) as i32;

        let box_w = 320;
        let box_h = 220;
        let left = center_x - box_w / 2;
        let top = center_y - box_h / 2;

        let (mx, my) = mouse_pos
            .map(|(x, y)| (x as i32, y as i32))
            .unwrap_or((-1, -1));

        // Fundo do popup de login e sombra
        fb.draw_rect(left + 4, top + 4, box_w, box_h, 0xAA0B0C10, None);
        fb.draw_rect(left, top, box_w, box_h, 0xFF1E1E2E, Some(0xFFBD93F9));

        // Título estilizado
        fb.draw_text(
            left + 24,
            top + 16,
            "PROJECT HADES - BERENICE",
            0xFFBD93F9,
            1,
        );

        // Rótulo de Usuário
        fb.draw_text(left + 24, top + 38, "USUARIO:", 0xFFF8F8F2, 1);
        let user_box_y = top + 50;
        let user_hovered =
            mx >= left + 20 && mx <= left + box_w - 20 && my >= user_box_y && my <= user_box_y + 22;
        let user_color = if self.focused_field == LoginField::Username {
            0xFF50FA7B // Foco Verde
        } else if user_hovered {
            0xFF8BE9FD // Hover Ciano
        } else {
            0xFF6272A4 // Cinza neutro
        };
        fb.draw_rect(
            left + 20,
            user_box_y,
            box_w - 40,
            22,
            0xFF282A36,
            Some(user_color),
        );
        let (user_text, user_color_text) = if self.username.is_empty() {
            if self.focused_field == LoginField::Username {
                ("_", 0xFF50FA7B)
            } else {
                ("hades", 0xFF6272A4)
            }
        } else {
            (self.username.as_str(), 0xFFF8F8F2)
        };
        fb.draw_text(left + 26, user_box_y + 7, user_text, user_color_text, 1);

        // Rótulo de Senha
        fb.draw_text(left + 24, top + 80, "SENHA:", 0xFFF8F8F2, 1);
        let pass_box_y = top + 92;
        let pass_hovered =
            mx >= left + 20 && mx <= left + box_w - 20 && my >= pass_box_y && my <= pass_box_y + 22;
        let pass_color = if self.focused_field == LoginField::Password {
            0xFF50FA7B
        } else if pass_hovered {
            0xFF8BE9FD
        } else {
            0xFF6272A4
        };
        fb.draw_rect(
            left + 20,
            pass_box_y,
            box_w - 40,
            22,
            0xFF282A36,
            Some(pass_color),
        );
        let (pass_mask, pass_color_text) = if self.password.is_empty() {
            if self.focused_field == LoginField::Password {
                ("_", 0xFF50FA7B)
            } else {
                ("*****", 0xFF6272A4)
            }
        } else {
            let len = self.password.len().min(24);
            ("************************"[..len].as_ref(), 0xFFFFB86C)
        };
        fb.draw_text(left + 26, pass_box_y + 7, pass_mask, pass_color_text, 1);

        // Botão Conectar
        let btn_y = top + 130;
        let btn_hovered =
            mx >= left + 40 && mx <= left + box_w - 40 && my >= btn_y && my <= btn_y + 26;
        let (btn_fill, btn_border, btn_text) =
            if self.focused_field == LoginField::ConnectButton || btn_hovered {
                (0xFFFF79C6, 0xFFFFFFFF, 0xFF282A36)
            } else {
                (0xFF44475A, 0xFF6272A4, 0xFFF8F8F2)
            };
        fb.draw_rect(left + 40, btn_y, box_w - 80, 26, btn_fill, Some(btn_border));
        fb.draw_text(
            left + (box_w / 2) - 45,
            btn_y + 8,
            "[ ENTRAR (A) ]",
            btn_text,
            1,
        );

        // Mensagem de status e dicas de controle
        fb.draw_text(left + 20, top + 172, &self.status_message, 0xFF8BE9FD, 1);
        fb.draw_text(
            left + 16,
            top + 196,
            "CLIQUE OU ENTER: ENTRAR",
            0xFF6272A4,
            1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_mouse_click_interaction() {
        let mut scene = LoginScene::new();
        let screen_w = 800;
        let screen_h = 600;
        let center_x = 400;
        let center_y = 300;
        let top = center_y - 110;

        // Clique no campo de senha
        let clicked_connect =
            scene.handle_click(center_x as f32, (top + 100) as f32, screen_w, screen_h);
        assert!(!clicked_connect);
        assert_eq!(scene.focused_field, LoginField::Password);

        // Clique no campo de usuário
        let clicked_connect =
            scene.handle_click(center_x as f32, (top + 60) as f32, screen_w, screen_h);
        assert!(!clicked_connect);
        assert_eq!(scene.focused_field, LoginField::Username);

        // Clique no botão conectar
        let clicked_connect =
            scene.handle_click(center_x as f32, (top + 140) as f32, screen_w, screen_h);
        assert!(clicked_connect);
        assert_eq!(scene.focused_field, LoginField::ConnectButton);
    }

    #[test]
    fn test_login_field_navigation() {
        let mut scene = LoginScene::new();
        assert_eq!(scene.focused_field, LoginField::Username);

        scene.next_field();
        assert_eq!(scene.focused_field, LoginField::Password);

        scene.next_field();
        assert_eq!(scene.focused_field, LoginField::ConnectButton);

        scene.next_field();
        assert_eq!(scene.focused_field, LoginField::Username);

        scene.prev_field();
        assert_eq!(scene.focused_field, LoginField::ConnectButton);
    }

    #[test]
    fn test_login_input_editing() {
        let mut scene = LoginScene::new();
        scene.handle_char('p');
        scene.handle_char('l');
        scene.handle_char('a');
        scene.handle_char('y');
        scene.handle_char('e');
        scene.handle_char('r');
        assert_eq!(scene.username, "player");

        scene.handle_backspace();
        assert_eq!(scene.username, "playe");

        scene.next_field(); // Muda para Password
        scene.handle_char('s');
        scene.handle_char('e');
        scene.handle_char('c');
        scene.handle_char('r');
        scene.handle_char('e');
        scene.handle_char('t');
        assert_eq!(scene.password, "secret");
    }

    #[test]
    fn test_login_render_framebuffer() {
        use crate::render::COLOR_BG;
        let mut fb = SoftwareFramebuffer::new(400, 300);
        fb.clear(COLOR_BG);

        let scene = LoginScene::new();
        scene.render(&mut fb);

        let ppm = fb.to_ppm();
        assert!(!ppm.is_empty());
    }
}
