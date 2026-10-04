//! SPEC-0006: Relógio e Motor de Ticks determinístico (20 Hz / 50ms por tick).
//!
//! Controla o pulso temporal do servidor sem flutuações e desacoplado de I/O.

pub const TICK_RATE_HZ: u32 = 20;
pub const TICK_DURATION_MILLIS: u64 = 1000 / (TICK_RATE_HZ as u64); // 50ms
pub const TICK_DURATION_MICROS: u64 = TICK_DURATION_MILLIS * 1000; // 50.000 micros

/// Identificador sequencial monotonicamente crescente do tick do servidor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub const fn new(val: u64) -> Self {
        Self(val)
    }

    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline(always)]
    pub const fn as_millis(self) -> u64 {
        self.0 * TICK_DURATION_MILLIS
    }

    #[inline(always)]
    pub fn as_seconds_f64(self) -> f64 {
        (self.0 as f64) / (TICK_RATE_HZ as f64)
    }

    #[inline(always)]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Relógio de simulação que avança ticks discretos determinísticos.
#[derive(Debug, Clone, Default)]
pub struct TickClock {
    current_tick: Tick,
}

impl TickClock {
    pub fn new() -> Self {
        Self {
            current_tick: Tick::ZERO,
        }
    }

    pub fn with_initial_tick(initial_tick: u64) -> Self {
        Self {
            current_tick: Tick::new(initial_tick),
        }
    }

    #[inline(always)]
    pub fn current_tick(&self) -> Tick {
        self.current_tick
    }

    /// Avança exatamente 1 tick determinístico.
    #[inline(always)]
    pub fn advance_tick(&mut self) -> Tick {
        self.current_tick = self.current_tick.next();
        self.current_tick
    }

    /// Reinicia o relógio.
    pub fn reset(&mut self) {
        self.current_tick = Tick::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_constants_and_timing() {
        assert_eq!(TICK_RATE_HZ, 20);
        assert_eq!(TICK_DURATION_MILLIS, 50);
        assert_eq!(TICK_DURATION_MICROS, 50_000);
    }

    #[test]
    fn test_tick_conversions() {
        let t20 = Tick::new(20);
        assert_eq!(t20.as_millis(), 1000); // 20 ticks * 50ms = 1 segundo
        assert!((t20.as_seconds_f64() - 1.0).abs() < f64::EPSILON);

        let t100 = Tick::new(100);
        assert_eq!(t100.as_millis(), 5000); // 5 segundos
    }

    #[test]
    fn test_tick_clock_advance() {
        let mut clock = TickClock::new();
        assert_eq!(clock.current_tick(), Tick::ZERO);

        for i in 1..=5 {
            assert_eq!(clock.advance_tick(), Tick::new(i));
        }
        assert_eq!(clock.current_tick(), Tick::new(5));
    }
}
