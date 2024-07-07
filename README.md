[![All release](https://github.com/PsykoDev/neko_sama_downloader/actions/workflows/rust.yml/badge.svg)](https://github.com/PsykoDev/neko_sama_downloader/actions/workflows/rust.yml)

if you got an error about chrome, remember to update chrome driver AND google chrome browser, if you've always an issue please open new issue with the message obtained
# Usage

![image](https://cdn.discordapp.com/attachments/762788506040795239/1257757464581898351/Screenshot_2024-07-02_at_19.59.04.png?ex=6685916b&is=66843feb&hm=3caa308e8b6b2ba55d904f32b6b3c1ae3cdf31037797f823ca44bb72ce040730&)

```txt
./neko_dl -s "anime name" -l <vf | vostfr> ( optional default=vf ) -t <worker number> ( optional default=1 )
./neko_dl -s "url" -t <worker number> ( optional default=1 )
./neko_dl --help
```

# Features

- config file to save default language & worker & save path
- Multi thread to download and process video
- Build vlc playlist at the end of process ( if contain 2 or more video )
- Can download only 1 episode or entire season ( based on url given )
- Can search all seasons by same name and download all seasons found
- ublock origin is added by default
- Can search then select what seasons you want, All or by unique id or multiple id
- ask if we continue when missing episode detected
- ask if already download or path detected 
- sequential download for little internet connexion to watch episode while downloading

# Note

- (Multi thread) 1 thread can download between 3 and 5 mo/s ( limited by website ) so 20 thread is good for 1gb/s fiber
- (Vlc Playlist) is based on path if you move all video download the playlist is broken
- (ublock origin) can't be disabled, it's a better way to stay safe
- (search engine) is not perfect but work
- for linux / macos ffmpeg need to be installed from pacman, apt, brew ... / for windows download the latest release asked by neko_dl if you got an error about it

# Actual Support

- only work with " https://neko-sama.fr/ " for now
- work on macOS windows linux

# Demo

- full demo : https://www.youtube.com/watch?v=hFqYkjqt0gs
![image](https://github.com/PsykoDev/neko_sama_downloader/assets/45910905/21c40853-f1fe-4c5c-9a25-9dab00e2f31d)
