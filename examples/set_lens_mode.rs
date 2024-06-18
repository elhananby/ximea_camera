use xiapi;

fn main() -> Result<(), i32> {
    let mut cam = xiapi::open_device(None)?;

    let mut retries = 5;
    let delay = std::time::Duration::from_secs(2);
    cam.set_lens_mode(xiapi::XI_SWITCH::XI_ON)?;
    println!("Lens mode set to XI_ON");

    loop {
        // match
        match cam.set_lens_aperture_value(5.2) {
            Ok(_) => {
                println!("Lens aperture value set to 5.2");
                break;
            }
            Err(e) => {
                println!("Error setting lens aperture value: {}, retrying.", e);
                retries -= 1;
                if retries == 0 {
                    println!("Max retries reached, exiting.");
                    return Err(e);
                }
                std::thread::sleep(delay);
                continue;
            }
        }
    }

    Ok(())
}
