mod app;
mod config;
mod ui;
mod utils;
mod video;

use app::App;

fn main() -> iced::Result {
    env_logger::init();
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .window(iced::window::Settings {
            size: iced::Size::new(960.0, 600.0),
            resizable: false,
            ..Default::default()
        })
        .run()
}
