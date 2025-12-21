use serde::{Deserialize, Serialize};

macro_rules! make_video_quality_format {
    (>  2160p) => { "bestvideo*[height>=4320]+bestaudio/best[height>=4320]" };
    (~> 2160p) => { concat!(make_video_quality_format!(~~ 2160p), "/", make_video_quality_format!(~> 1440p)) };
    (~> 1440p) => { concat!(make_video_quality_format!(~~ 1440p), "/", make_video_quality_format!(~> 1080p)) };
    (~> 1080p) => { concat!(make_video_quality_format!(~~ 1080p), "/", make_video_quality_format!(~>  720p)) };
    (~>  720p) => { concat!(make_video_quality_format!(~~  720p), "/", make_video_quality_format!(~>  480p)) };
    (~>  480p) => { concat!(make_video_quality_format!(~~  480p), "/", make_video_quality_format!(~>  320p)) };
    (~>  320p) => { concat!(make_video_quality_format!(~~  320p), "/", make_video_quality_format!(~>  240p)) };
    (~>  240p) => { concat!(make_video_quality_format!(~~  240p), "/", make_video_quality_format!(~>  144p)) };
    (~>  144p) => { concat!(make_video_quality_format!(~~  144p), "/", make_video_quality_format!(=fallback)) };

    (~~ 2160p) => { "bestvideo*[height>2160][height<4320]+bestaudio/best[height>2160][height<4320]/bestvideo*[height=2160]+bestaudio/best[height=2160]" };
    (~~ 1440p) => { "bestvideo*[height>1440][height<2160]+bestaudio/best[height>1440][height<2160]/bestvideo*[height=1440]+bestaudio/best[height=1440]" };
    (~~ 1080p) => { "bestvideo*[height>1080][height<1440]+bestaudio/best[height>1080][height<1440]/bestvideo*[height=1080]+bestaudio/best[height=1080]" };
    (~~  720p) => { "bestvideo*[height>720][height<1080]+bestaudio/best[height>720][height<1080]/bestvideo*[height=720]+bestaudio/best[height=720]" };
    (~~  480p) => { "bestvideo*[height>480][height<720]+bestaudio/best[height>480][height<720]/bestvideo*[height=480]+bestaudio/best[height=480]" };
    (~~  320p) => { "bestvideo*[height>320][height<480]+bestaudio/best[height>320][height<480]/bestvideo*[height=320]+bestaudio/best[height=320]" };
    (~~  240p) => { "bestvideo*[height>240][height<320]+bestaudio/best[height>240][height<320]/bestvideo*[height=240]+bestaudio/best[height=240]" };
    (~~  144p) => { "bestvideo*[height>144][height<240]+bestaudio/best[height>144][height<240]/bestvideo*[height=144]+bestaudio/best[height=144]" };
    (=fallback) => { "bestvideo*+bestaudio/bestvideo+bestaudio/best" };
}

pub static DEFAULT_GOOD_VIDEO_QUALITY: VideoQuality = VideoQuality::Best1080p;

#[derive(clap::ValueEnum, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum VideoQuality {
    AbsoluteBest,
    Best2160p,
    Best1440p,
    Best1080p,
    Best720p,
    Best480p,
    Best320p,
    Best240p,
    Best144p,
}

impl VideoQuality {
    pub fn to_yt_dlp_format(self) -> &'static str {
        match self {
            VideoQuality::AbsoluteBest => make_video_quality_format!(> 2160p),
            VideoQuality::Best2160p => make_video_quality_format!(~> 2160p),
            VideoQuality::Best1440p => make_video_quality_format!(~> 1440p),
            VideoQuality::Best1080p => make_video_quality_format!(~> 1080p),
            VideoQuality::Best720p => make_video_quality_format!(~> 720p),
            VideoQuality::Best480p => make_video_quality_format!(~> 480p),
            VideoQuality::Best320p => make_video_quality_format!(~> 320p),
            VideoQuality::Best240p => make_video_quality_format!(~> 240p),
            VideoQuality::Best144p => make_video_quality_format!(~> 144p),
        }
    }
}
