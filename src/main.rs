mod device;
mod gif_io;
mod paths;
mod protocol;
mod remap;
mod studio;

fn main() -> eframe::Result {
    studio::run()
}
