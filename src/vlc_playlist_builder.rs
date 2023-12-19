use std::error::Error;
use std::fs::File;

use quick_xml::Writer;

pub fn _new() -> Result<(), Box<dyn Error>> {
    let file = File::create("playlist.xml").expect("Impossible de créer le fichier XML");
    let mut writer = Writer::new(file);

    let x = writer.create_element("playlist");
    x.with_attributes(
        vec![
            ("xmlns", "http://xspf.org/ns/0/"),
            ("xmlns:vlc", "http://www.videolan.org/vlc/playlist/ns/0/"),
            ("version", "1"),
        ]
            .into_iter(),
    );

    Ok(())
}
