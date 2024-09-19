use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::{ thread::sleep, time::Duration };

use crate::string;

lazy_static! {
    pub(crate) static ref TUTORIAL: Mutex<bool> = Mutex::new(false);
    pub(crate) static ref TUTORIAL_CHAPTER_INFO: Mutex<bool> = Mutex::new(true);
    pub(crate) static ref TUTORIAL_CHAPTER: Mutex<bool> = Mutex::new(true);
}

pub(crate) fn manga_info() {
    let message = "<--- This is the status of manga information";
    string(2, 0, &"-".repeat(32));
    sleep(Duration::from_millis(100));
    string(1, 31, message);
    sleep(Duration::from_secs(3));
    string(2, 0, &" ".repeat(32));
    sleep(Duration::from_millis(100));
    string(1, 31, &" ".repeat(message.len()));
}

pub(crate) fn cover_art() {
    let message = "<--- This is the downloading process of cover art";
    string(3, 0, &"-".repeat(24));
    sleep(Duration::from_millis(100));
    string(2, 23, message);
    sleep(Duration::from_secs(3));
    string(3, 0, &" ".repeat(24));
    sleep(Duration::from_millis(100));
    string(2, 23, &" ".repeat(message.len()));
}

pub(crate) fn feed(stat: u32) {
    let message = "<--- This is the downloading process of manga feed information";
    string(4 + stat, 0, &"-".repeat(31));
    sleep(Duration::from_millis(100));
    string(3 + stat, 30, message);
    sleep(Duration::from_secs(3));
    string(4 + stat, 0, &" ".repeat(31));
    sleep(Duration::from_millis(100));
    string(3 + stat, 30, &" ".repeat(message.len()));
}

pub(crate) fn found_chapter() {
    let message = "<--- This is the id of current chapter that program is parsing";
    string(2, 0, &"-".repeat(66));
    sleep(Duration::from_millis(100));
    string(1, 65, message);
    sleep(Duration::from_secs(3));
    string(2, 0, &" ".repeat(66));
    sleep(Duration::from_millis(100));
    string(1, 65, &" ".repeat(message.len()));
}

pub(crate) fn skip() {
    let message = "^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^";
    let message1 = "Reason for skipping this chapter";
    string(3, 8, message);
    sleep(Duration::from_millis(100));
    string(4, 8, message1);
    sleep(Duration::from_secs(3));
    string(3, 8, &" ".repeat(message.len()));
    sleep(Duration::from_millis(100));
    string(4, 8, &" ".repeat(message1.len()));
}

pub(crate) fn metadata() {
    let message = "^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^";
    let message1 = "Metadata of chapter that is going to get downloaded";
    string(3, 0, message);
    sleep(Duration::from_millis(100));
    string(4, 0, message1);
    sleep(Duration::from_secs(3));
    string(3, 0, &" ".repeat(message.len()));
    sleep(Duration::from_millis(100));
    string(4, 0, &" ".repeat(message1.len()));
}

pub(crate) fn chapter_info() {
    let message = "^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^";
    let message1 = "This is the downloading process of manga chapter information";
    string(4, 0, message);
    sleep(Duration::from_millis(100));
    string(5, 0, message1);
    sleep(Duration::from_secs(3));
    string(4, 0, &" ".repeat(message.len()));
    sleep(Duration::from_millis(100));
    string(5, 0, &" ".repeat(message1.len()));
}

pub(crate) fn images() {
    let message = "Here is area of all images download processes";
    string(8, 20, message);
    sleep(Duration::from_secs(3));
    string(8, 20, &" ".repeat(message.len()));
}
