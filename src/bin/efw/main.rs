pub mod efw;
use env_logger::Env;

fn main() {
    let env = Env::default().filter_or("LS_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    efw::look_for_devices();
}
