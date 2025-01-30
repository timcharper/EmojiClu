pub mod clue_generator;
mod clue_generator_state;
pub mod game_state;
mod puzzle_variants;
pub mod settings;
pub mod solver;
pub mod stats_manager;

pub use clue_generator::generate_clues;
pub use solver::deduce_clue;

#[cfg(test)]
pub mod tests {
    use test_context::TestContext;

    pub struct UsingLogger {
        _value: String,
    }

    impl TestContext for UsingLogger {
        fn setup() -> UsingLogger {
            env_logger::init();
            UsingLogger {
                _value: "Hello, World!".to_string(),
            }
        }

        fn teardown(self) {
            // Perform any teardown you wish.
        }
    }
}
