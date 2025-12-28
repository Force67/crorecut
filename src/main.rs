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

fn main() -> iced::Result {
    env_logger::init();

    let icon = window::icon::from_file_data(icons::WINDOW_ICON, Some(ImageFormat::Png))
        .expect("Failed to load window icon");

    iced::application(Crorecut::new, Crorecut::update, Crorecut::view)
        .title(Crorecut::title)
        .style(Crorecut::style)
        .subscription(Crorecut::subscription)
        .settings(iced::Settings {
            vsync: false,
            ..Default::default()
        })
        .window(window::Settings {
            size: iced::Size::new(960.0, 600.0),
            min_size: Some(iced::Size::new(800.0, 540.0)),
            icon: Some(icon),
            ..Default::default()
        })
        .run()
}
