use std::process::Command;

fn main() {
    println!("Executing build script to build static files");

    match Command::new("make")
        .arg("backend/static/index.html")
        .current_dir("../")
        .status()
    {
        Ok(_) => {
            println!("Build script finished successfully");
            std::process::exit(0);
        }
        Err(_) => {
            println!("Build script failed");
            std::process::exit(1);
        }
    }
}
