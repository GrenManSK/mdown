use crate::{ utils, error::mdown::Error };

fn app() -> Result<(), eframe::Error> {
    Ok(())
}

pub(crate) fn start() -> Result<(), Error> {
    match app() {
        Ok(()) => (),
        Err(err) => eprintln!("Error gui: {}", err),
    }

    match utils::remove_cache() {
        Ok(()) => (),
        Err(err) => {
            return Err(err);
        }
    }
    Ok(())
}
