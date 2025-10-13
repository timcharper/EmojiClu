mod destroyable;
pub mod events;
pub mod game;
pub mod helpers;
pub mod model;
pub mod solver;
pub mod ui;

fluent_i18n::i18n!("locales", fallback = "en");

#[cfg(test)]
mod tests {
    use std::sync::Once;
    use test_context::TestContext;

    static INIT_LOGGER: Once = Once::new();

    pub struct UsingLogger {
        _value: String,
    }

    impl TestContext for UsingLogger {
        fn setup() -> UsingLogger {
            INIT_LOGGER.call_once(|| {
                env_logger::init();
            });

            UsingLogger {
                _value: "Hello, World!".to_string(),
            }
        }

        fn teardown(self) {
            // Perform any teardown you wish.
        }
    }
}
