use ksni::{MenuItem, Tray, menu::StandardItem};
use std::sync::mpsc::Sender;

pub enum Event {
    Show,
    Quit,
    Offline,
    Online,
}

pub struct CatTray(pub Sender<Event>);

impl Tray for CatTray {
    fn id(&self) -> String {
        "com.workercat.catdo".into()
    }
    fn title(&self) -> String {
        "CatDo".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let Ok(image) =
            image::load_from_memory(include_bytes!("../../../assets/com.workercat.catdo.png"))
        else {
            return vec![];
        };
        let image = image
            .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
            .into_rgba8();
        vec![ksni::Icon {
            width: 64,
            height: 64,
            data: image
                .pixels()
                .flat_map(|pixel| [pixel[3], pixel[0], pixel[1], pixel[2]])
                .collect(),
        }]
    }
    fn activate(&mut self, _: i32, _: i32) {
        let _ = self.0.send(Event::Show);
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Open CatDo".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.0.send(Event::Show);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit CatDo".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.0.send(Event::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
    fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
        let _ = self.0.send(Event::Offline);
        true
    }
    fn watcher_online(&self) {
        let _ = self.0.send(Event::Online);
    }
}
