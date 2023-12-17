use std::error::Error;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::Writer;
use std::fs::File;
use std::io::Write;

pub fn new() -> Result<(), Box<dyn Error>>{
	let file = File::create("playlist.xml").expect("Impossible de créer le fichier XML");
	let mut writer = Writer::new(file);

	let x =writer.create_element("playlist");
	x.with_attributes(vec![("xmlns", "http://xspf.org/ns/0/"), ("xmlns:vlc", "http://www.videolan.org/vlc/playlist/ns/0/"), ("version", "1")].into_iter());


	Ok(())
}



/*

		<track>
			<location>file:///C:/Users/blap/Downloads/Video/Fate-Apocrypha%2024%20VF%20-%20Neko%20Sama.ts</location>
			<duration>1421962</duration>
			<extension application="http://www.videolan.org/vlc/playlist/0">
				<vlc:id>23</vlc:id>
			</extension>
		</track>

 */