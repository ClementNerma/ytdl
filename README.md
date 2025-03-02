# Youtube Downloader

This program simplifies downloading of videos and playlists from various platform. It acts as a high-level wrapper for [`yt-dlp`](https://github.com/yt-dlp/yt-dlp), which is a fork of [`youtube-dl`](https://github.com/yt-dlp/yt-dlp).

Note that this program requires pretty heaving configuration, and is tailored for more advanced use cases (see [Configuration](#configuration)).

It fits in a single binary with no external dependency other than `yt-dlp` itself.

A few features are:

* Always automatically downloads the very highest quality available (highest resolution, best codec, best audio codec and bitrate)
* Auto-remuxing when downloading non-matching video and audio streams
* Automatic thumbnails downloading and embedding
* Nice UI showing informations about the videos to download
* Auto-retry in case of failure
* Synchronization of playlists on disk
* Automatic blacklisting of unavilable videos
* Ability to add some metadata to the downloaded files (e.g. modification time based on the video's upload date)
* Download of music albums (from supported platforms)

## Install

There are various ways to install `ytdl` on your machine:

* Grab the latest binary from the [releases page](https://github.com/ClementNerma/ytdl)
* Install with [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall): `cargo binstall --git https://github.com/ClementNerma/ytdl ytdl`
* Install from source with `cargo install --git https://github.com/ClementNerma/ytdl`

The super basic usage is running `ytdl <video URL> --no-platform --skip-repair-date`. This is not the main intended use case, as you will see below.

## Configuration

To enable all features from `ytdl`, let's start by creating a configuration file with `ytdl init-config`. Then, in your favorite editor open `~/.config/ytdl/ytdl-config.json`. It should look like this:

```json
{
    "yt_dlp_bin": "yt-dlp",
    "tmp_dir": "/tmp/ytdl",
    "url_filename": ".ytdlsync-url",
    "cache_filename": ".ytdlsync-cache",
    "auto_blacklist_filename": ".ytdlsync-blacklist",
    "custom_blacklist_filename": ".ytdlsync-custom-blacklist",
    "default_bandwidth_limit": null,
    "platforms": {},
}
```

This contains all kind of informations about how ytdl is going to work.

The `yt_dlp_bin` should be the absolute path to your `yt-dlp` binary, or it can just be `yt-dlp` if it is in your `PATH`.

The `tmp_dir` is the directory that will be used to download videos before moving them to their final destination.

For now, we need to add some _platforms_, which indicates how videos should be downloaded. Here is the entry for Youtube:

```json
{
    // ...
    "platforms": {
        "Youtube": {
            "platform_url_matcher": "Start https www \"youtube.com/\"",
            "playlist_url_matchers": [
                "Start https www \"youtube.com/playlist?list=\" :id(id) End",
                "Start https www \"youtube.com/\" ('@' | ('c' | \"channel\") '/') :id(id) \"/videos\" End"
            ],
            "videos_url_regex": "Start https www \"youtube.com/\" (\"watch?v=\" | \"shorts/\") :id(id) End",
            "videos_url_prefix": "https://www.youtube.com/watch?v=",
            "dl_options": {}
        }
    }
}
```

Phew, this is pretty complex! Let's break it down.

Each platform has a `platform_url_matcher` which allows the program to know which platform we're downloading from. It is a [Pomsky](https://pomsky-lang.org/) regular expression with a few builtin variables, like `https`  which matches either `http://` or `https://`. Then we have `www` which optionally matches `www.`.

When we download a video from Youtube, such as [`https://www.youtube.com/watch?v=dQw4w9WgXcQ`](https://www.youtube.com/watch?v=dQw4w9WgXcQ), the regex will match on the URL as it starts with `https://www.youtube.com/`. This allows `ytdl` to know we're downloading from Youtube and that it should follow the configuration written in the corresponding profile.

Then we have the `playlist_url_matchers`, which match and extract the identifier from playlist URLs. The `id` variable allows to match basically any character. For instance, the playlist at [`https://www.youtube.com/playlist?list=PLp_G0HWfCo5raQSCb_BxY6oA1OVnNBolc`](https://www.youtube.com/playlist?list=PLp_G0HWfCo5raQSCb_BxY6oA1OVnNBolc) will be recognized by our matcher and the extracted ID will be `PLp_G0HWfCo5raQSCb_BxY6oA1OVnNBolc`.

The second regex matches channel URLs, which are playlists too!

Next we have `videos_url_regex` which does the same as playlists, but for videos this time.

There's also `videos_url_prefix` which allows to build an URL from an ID. The basic idea is that ytdl works with videos and playlists IDs, and then reconstruct the URLs afterwards.

Finally, we have `dl_options` which contains - as you guessed - options related to the downloading itself. All parameters inside it are optional, and include:

| Option name            | Example value | Description                                                                                                                                                |
| ---------------------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bandwidth_limit`      | `25M`         | Maximum download speed (per second)                                                                                                                        |
| `needs_checking`       | `true`        | Indicates if all videos should be checked for availibility. Required for some platforms                                                                    |
| `rate_limited`         | `true`        | Indicates if the platform applies heavy rate-limiting. Disables parallel fetching of informations to make it sequential instead                            |
| `cookies_from_browser` | `firefox`     | Allows to use the cookies from an existing browser. Required to access private videos or to get the highest quality on some platforms (e.g. Youtube Music) |
| `skip_repair_date`     | `true`        | Don't repair the date (see below)                                                                                                                          |
| `output_format`        | `mkv`         | Put the downloaded video in a specific format                                                                                                              |
| `download_format`      | `bestaudio`   | Force a specific preset from `yt-dlp`                                                                                                                      |
| `no_thumbnail`         | `true`        | Disable thumbnail downloading and embedding                                                                                                                |

## Usage

We can now start downloading some videos!

```shell
# Download a video
ytdl dl 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'

# Download a playlist
ytdl dl 'https://www.youtube.com/playlist?list=PLp_G0HWfCo5raQSCb_BxY6oA1OVnNBolc'
```

By default, this will also _repair_ the date, which means it will fetch the video's upload date from the platform and store it as the file's modification time.

There are lots of options, you can check them with `ytdl dl --help`.

## Synchronizing playlists

A neat feature of `ytdl` is the ability to _synchronize_ playlists. Basically, you set up a folder to store all videos from a given playlist, and when you run a specific command, it will only download the videos that aren't in the folder yet.

```shell
# Create a directory for the downloads
mkdir rick
cd rick/

# Setup a playlist for the current directory
# This will by default create a file named `.ytdlsync-url`
ytdl sync setup 'https://www.youtube.com/playlist?list=PLp_G0HWfCo5raQSCb_BxY6oA1OVnNBolc'

# This will fetch all infos from the playlist and create a file named by default `.ytdlsync-cache`
ytdl sync run
```

The program will fetch the playlist's content, and only download videos that aren't in the directory. It will recognize them by extracting the ID from the downloaded files' name.

If the process is interrupted, you can re-run it and it won't have to fetch the playlist's infos as they are cached on disk. You can delete the cache file manually if you wish to force fetching the entire playlist anyway.

### Manual blacklisting

If you don't want to download a specific video for whatever reason, you can _blacklist_ it:

```shell
# We put the platform's name and then the video's ID
ytdl sync blacklist Youtube dQw4w9WgXcQ
```

This one will be removed from the list of videos to download. They will be put in the configured file, by default `.ytdlsync-custom-blacklist`.

### Automatic blacklisting

If a video is marked as unavailable by the platform (e.g. a deleted video on Youtube), it will be automatically blacklisted and put in a file named by default `.ytdlsync-blacklist`.

## Albums downloading

It is possible to download music albums from supported platforms, such as Youtube Music.

Let's start by adding the relevant profile to our configuration file:

```json
{
    // ...
    "platforms": {
        "Youtube": { /* ... */ },

        "Youtube Music": {
            "platform_url_matcher": "Start https \"music.youtube.com/\"",
            "playlist_url_matchers": [
                "Start https \"music.youtube.com/playlist?list=\" :id(id) End",
                "Start https \"music.youtube.com/browse/\" :id(id) End"
            ],
            "videos_url_regex": "Start https \"music.youtube.com/\" (\"watch?v=\") :id(id) (\"&\" | End)",
            "videos_url_prefix": "https://music.youtube.com/watch?v=",
            "dl_options": {
                "download_format": "bestaudio",
                "bandwidth_limit": "25M",
                "no_thumbnail": true,
                "redirect_playlist_videos": true
            }
        }
    }
}
```

Next we can download any album:

```shell
ytdl album 'https://music.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE'
```

If you are logged into Youtube Music in your browser and want to use the cookies to get the higher stream quality:

```shell
ytdl album 'https://music.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE' --cookies-from-browser firefox # or chrome, etc.
```
