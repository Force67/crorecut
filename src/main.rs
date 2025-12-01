mod app;
mod config;
mod ui;
mod utils;
mod video;

use app::App;

fn main() -> iced::Result {
    env_logger::init();
    iced::application(App::title, App::update, App::view).run_with(App::new)
}
