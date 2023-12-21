use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

pub fn new(track: Vec<(PathBuf, &String)>) -> Result<(), Box<dyn Error>> {
    let save_path = track.first().unwrap().0.parent().unwrap();
    let save_name = track.first().unwrap().1;
    let file = File::create(format!("{}/{}.xspf", save_path.display(), save_name))?;
    let mut writer = Writer::new(file);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    writer.write_event(Event::Start(BytesStart::new("playlist").with_attributes(
        vec![
            ("xmlns", "http://xspf.org/ns/0/"),
            ("xmlns:vlc", "http://www.videolan.org/vlc/playlist/ns/0/"),
            ("version", "1"),
        ],
    )))?;
    write_element(&mut writer, "title", "Liste de lecture")?;
    writer.write_event(Event::Start(BytesStart::new("trackList")))?;

    for (id, path) in track.iter().enumerate() {
        write_track(
            &mut writer,
            id,
            path_to_vlcpath(path.0.to_str().unwrap()).as_str(),
        )?;
    }

    writer.write_event(Event::End(BytesEnd::new("trackList")))?;
    writer.write_event(Event::Start(BytesStart::new("extension").with_attributes(
        vec![("application", "http://www.videolan.org/vlc/playlist/0")],
    )))?;

    for (id, _) in track.iter().enumerate() {
        write_vlc_item(&mut writer, id)?;
    }
    writer.write_event(Event::End(BytesEnd::new("extension")))?;
    writer.write_event(Event::End(BytesEnd::new("playlist")))?;
    Ok(())
}

fn write_element(writer: &mut Writer<File>, tag: &str, text: &str) -> Result<(), Box<dyn Error>> {
    writer.write_event(Event::Start(BytesStart::new(tag)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(tag)))?;
    Ok(())
}

fn write_track(writer: &mut Writer<File>, id: usize, location: &str) -> Result<(), Box<dyn Error>> {
    writer.write_event(Event::Start(BytesStart::new("track")))?;
    write_element(writer, "location", location)?;
    write_element(writer, "duration", "0")?;
    writer.write_event(Event::Start(BytesStart::new("extension").with_attributes(
        vec![("application", "http://www.videolan.org/vlc/playlist/0")],
    )))?;
    write_element(writer, "vlc:id", &id.to_string())?;
    writer.write_event(Event::End(BytesEnd::new("extension")))?;
    writer.write_event(Event::End(BytesEnd::new("track")))?;
    Ok(())
}

fn write_vlc_item(writer: &mut Writer<File>, tid: usize) -> Result<(), Box<dyn Error>> {
    let element_start =
        BytesStart::new("vlc:item").with_attributes(vec![("tid", tid.to_string().as_str())]);
    writer.write_event(Event::Empty(element_start))?;
    Ok(())
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "linux")]
fn path_to_vlcpath(path: &str) -> String {
    format!("file://{}", path.replace(" ", "%20"))
}

#[cfg(target_os = "windows")]
fn path_to_vlcpath(path: &str) -> String {
    format!("file:///{}", path.replace(" ", "%20"))
}
