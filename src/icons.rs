use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Icons;
impl AssetSource for Icons {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path == "brand" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/flacpac-face.svg"
            ))));
        }
        let shape = match path {
            "clock" => "<circle cx='12' cy='12' r='9'/><path d='M12 7v5l3 2'/>",
            "menu" => {
                "<circle cx='5' cy='12' r='1'/><circle cx='12' cy='12' r='1'/><circle cx='19' cy='12' r='1'/>"
            }
            "more" => {
                "<circle cx='5' cy='12' r='1'/><circle cx='12' cy='12' r='1'/><circle cx='19' cy='12' r='1'/>"
            }
            "collapse" => "<path d='M15 4v16M9 8l-4 4 4 4'/>",
            "expand" => "<path d='M9 4v16M15 8l4 4-4 4'/>",
            "settings" => {
                "<circle cx='12' cy='12' r='3'/><path d='m19.4 15 .1.1 1.4 1.1-1.4 2.4-1.7-.7a8 8 0 0 1-1.6.9l-.3 1.8h-2.8l-.3-1.8a8 8 0 0 1-1.6-.9l-1.7.7-1.4-2.4 1.4-1.1a7 7 0 0 1 0-1.9l-1.4-1.1 1.4-2.4 1.7.7a8 8 0 0 1 1.6-.9l.3-1.8h2.8l.3 1.8a8 8 0 0 1 1.6.9l1.7-.7 1.4 2.4-1.4 1.1a7 7 0 0 1 0 1.9z' transform='translate(-1 -1)'/>"
            }
            "statistics" => "<path d='M4 19V5M4 19h16M8 16v-5M12 16V7M16 16v-8M20 16v-4'/>",
            "check" => "<path d='m5 12 4 4L19 6'/>",
            "drag" => {
                "<circle cx='9' cy='6' r='1'/><circle cx='15' cy='6' r='1'/><circle cx='9' cy='12' r='1'/><circle cx='15' cy='12' r='1'/><circle cx='9' cy='18' r='1'/><circle cx='15' cy='18' r='1'/>"
            }
            "shuffle" => {
                "<path d='M3 6h3c5 0 7 12 12 12h3M17 14l4 4-4 4M3 18h3c2 0 4-3 6-6s4-6 6-6h3M17 2l4 4-4 4'/>"
            }
            "repeat" => {
                "<path d='m17 2 4 4-4 4M3 11V8a2 2 0 0 1 2-2h16M7 22l-4-4 4-4M21 13v3a2 2 0 0 1-2 2H3'/>"
            }
            "repeat-one" => {
                "<path d='m17 2 4 4-4 4M3 11V8a2 2 0 0 1 2-2h16M7 22l-4-4 4-4M21 13v3a2 2 0 0 1-2 2H3M11 10l1-1v6'/>"
            }
            "grid" => {
                "<rect x='3' y='3' width='7' height='7' rx='1'/><rect x='14' y='3' width='7' height='7' rx='1'/><rect x='3' y='14' width='7' height='7' rx='1'/><rect x='14' y='14' width='7' height='7' rx='1'/>"
            }
            "list" => "<path d='M8 5h13M8 12h13M8 19h13M3 5h.01M3 12h.01M3 19h.01'/>",
            "play" => "<path d='m8 5 11 7-11 7z' fill='white' stroke='none'/>",
            "pause" => "<path d='M8 5v14M16 5v14' stroke-width='4'/>",
            "next" => "<path d='m5 5 10 7-10 7z' fill='white' stroke='none'/><path d='M19 5v14'/>",
            "previous" => {
                "<path d='m19 5-10 7 10 7z' fill='white' stroke='none'/><path d='M5 5v14'/>"
            }
            "search" => "<circle cx='10.5' cy='10.5' r='6.5'/><path d='m16 16 5 5'/>",
            "plus" => "<path d='M12 5v14M5 12h14'/>",
            "close" => "<path d='m6 6 12 12M6 18 18 6'/>",
            "forward" => "<path d='m10 5 7 7-7 7'/>",
            "back" => "<path d='m14 5-7 7 7 7'/>",
            "music" => {
                "<path d='M9 18V5l12-2v13M9 8l12-2'/><ellipse cx='6' cy='18' rx='3' ry='3'/><ellipse cx='18' cy='16' rx='3' ry='3'/>"
            }
            "album" => {
                "<rect x='3' y='3' width='18' height='18' rx='3'/><circle cx='12' cy='12' r='5'/><circle cx='12' cy='12' r='1'/>"
            }
            "artist" => "<circle cx='12' cy='8' r='4'/><path d='M4 21v-2a8 8 0 0 1 16 0v2'/>",
            "queue" => {
                "<path d='M4 5h16M4 10h16M4 15h8M4 20h8'/><path d='m17 15 5 3-5 3z' fill='white' stroke='none'/>"
            }
            "folder" => {
                "<path d='M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'/>"
            }
            "volume" => {
                "<path d='M3 9h4l5-4v14l-5-4H3zM16 8a6 6 0 0 1 0 8M19 5a10 10 0 0 1 0 14'/>"
            }
            "sun" => {
                "<circle cx='12' cy='12' r='4'/><path d='M12 1v2M12 21v2M1 12h2M21 12h2M4 4l2 2M18 18l2 2M4 20l2-2M18 6l2-2'/>"
            }
            "moon" => "<path d='M21 13A9 9 0 0 1 11 3a9 9 0 1 0 10 10z'/>",
            "up" => "<path d='m6 15 6-6 6 6'/>",
            "down" => "<path d='m6 9 6 6 6-6'/>",
            _ => return Ok(None),
        };
        Ok(Some(Cow::Owned(format!("<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24' fill='none' stroke='white' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'>{shape}</svg>").into_bytes())))
    }
    fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
