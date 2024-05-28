use crate::{ error::MdownError, resolute, utils };

fn app() -> Result<(), eframe::Error> {
    Ok(())
}

pub(crate) fn start() -> Result<(), MdownError> {
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
    *resolute::FINAL_END.lock() = true;
    Ok(())
}
