use crate::{ error::mdown::Error, resolute, utils };

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
    *(match resolute::FINAL_END.lock() {
        Ok(value) => value,
        Err(err) => {
            return Err(Error::PoisonError(err.to_string()));
        }
    }) = true;
    Ok(())
}
