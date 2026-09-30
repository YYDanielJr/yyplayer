#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod bootstrap;
mod controller;
mod demo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap::run()
}
