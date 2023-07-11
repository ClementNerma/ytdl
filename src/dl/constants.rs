pub static DEFAULT_BEST_VIDEO_FORMAT: &str = concat!(
    "bestvideo*[height>=4320]+bestaudio/best[height>=4320]/",
    "bestvideo*[height>2160][height<4320]+bestaudio/best[height>2160][height<4320]/bestvideo*[height=2160]+bestaudio/best[height=2160]/",
    "bestvideo*[height>1440][height<2160]+bestaudio/best[height>1440][height<2160]/bestvideo*[height=1440]+bestaudio/best[height=1440]/",
    "bestvideo*[height>1080][height<1440]+bestaudio/best[height>1080][height<1440]/bestvideo*[height=1080]+bestaudio/best[height=1080]/",
    "bestvideo*[height>720][height<1080]+bestaudio/best[height>720][height<1080]/bestvideo*[height=720]+bestaudio/best[height=720]/",
    "bestvideo*[height>480]+bestaudio/best[height>480]/bestvideo*[height=480]+bestaudio/best[height=480]/",
    "bestvideo*[height>320]+bestaudio/best[height>320]/bestvideo*[height=320]+bestaudio/best[height=320]/",
    "bestvideo*[height>240]+bestaudio/best[height>240]/bestvideo*[height=240]+bestaudio/best[height=240]/",
    "bestvideo*[height>144]+bestaudio/best[height>144]/bestvideo*[height=144]+bestaudio/best[height=144]/",
    "bestvideo+bestaudio/",
    "best"
);

pub static DEFAULT_FILENAMING: &str = "%(title)s-%(id)s.%(ext)s";
