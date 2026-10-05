#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Created,
    Running,
    ShuttingDown,
}

#[derive(Debug)]
pub struct Engine {
    state: EngineState,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: EngineState::Created,
        }
    }

    pub fn start(&mut self) {
        self.state = EngineState::Running;
    }

    pub fn state(&self) -> EngineState {
        self.state
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_starts_in_created_state() {
        assert_eq!(Engine::new().state(), EngineState::Created);
    }

    #[test]
    fn engine_can_start() {
        let mut engine = Engine::new();
        engine.start();
        assert_eq!(engine.state(), EngineState::Running);
    }
}
