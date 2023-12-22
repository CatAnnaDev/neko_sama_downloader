# Usage
```txt
./anime_dl search "anime name" <vf | vostfr> ( optional default=vf ) <thread number> ( optional default=1 )
./anime_dl download "url" <thread number> ( optional default=1 )
./anime_dl help
```

# Features
  - Multi thread to download and process video 
  - Build vlc playlit at the end of process ( if contain 2 or more video )
  - Can download only 1 episode or entire season ( based on url given )
  - Can search all seasons by name and download 
  - ublock origin is added by default

# Note
- (Multi thread) 1 thread can download between 3 and 5 mo/s ( limited by website ) so 20 thread is good for 1gb/s fiber
- (Vlc Playlist) is based on path if you move all video download the playlist is broken
- (ublock origin) can't be disabled, it's a better way to stay safe
- (search engine) is not perfect but work 

# Actual Support
 - only work with " https://neko-sama.fr/ " for now
 - work on macOS windows linux

# Demo
- Download season: https://youtu.be/n_AlCJcqX6Q
- Download with search: https://youtu.be/mkgSpIDFO6Y

![](https://github.com/PsykoDev/neko_sama_downloader/assets/45910905/fe517de7-d7cc-4657-a03e-79c7f29883fa)

