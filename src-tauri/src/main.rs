// Impede que uma janela de console adicional apareça no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    markforge_lib::run();
}
