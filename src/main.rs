#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
// ^ Hide console in release builds

mod app;
mod audio;
mod icons;
mod ui;
mod utils;
mod video;

use app::Crorecut;
use iced::window;
use image::ImageFormat;

fn window_icon() -> iced::window::Icon {
    let mut icon_reader = image::ImageReader::new(std::io::Cursor::new(icons::WINDOW_ICON));
    icon_reader.set_format(ImageFormat::Png);
    let pixels = icon_reader.decode().unwrap().to_rgba8();

    iced::window::icon::from_rgba(pixels.to_vec(), pixels.width(), pixels.height()).unwrap()
}

fn main() -> iced::Result {
    env_logger::init();

    iced::application(Crorecut::new, Crorecut::update, Crorecut::view)
        .title(Crorecut::title)
        .style(Crorecut::style)
        .subscription(Crorecut::subscription)
        .settings(iced::Settings {
            vsync: false,
            ..Default::default()
        })
        .window(window::Settings {
            size: iced::Size::new(960., 860.),
            min_size: Some(iced::Size::new(800., 540.)),
            icon: Some(window_icon()),
            ..Default::default()
        })
        .run()
}
