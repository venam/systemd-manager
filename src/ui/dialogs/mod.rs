use gtk::*;

pub trait Dialogs {
    fn error_dialog(&self, msg: &str);
}

impl Dialogs for Window {
    fn error_dialog(&self, msg: &str) {
        let dialog = Dialog::new_with_buttons(
            "Systemd Manager: An Error Occurred".into(),
            self.into(),
            // TODO: Use DialogFlags::DESTROY_WITH_PARENT
            DialogFlags::empty(),
            &[]
        );

        let message = TextBuffer::new(None);
        message.set_text(msg);
        let message = TextView::new_with_buffer(&message);
        message.set_editable(false);
        message.set_wrap_mode(WrapMode::Word);
        message.set_border_width(5);

        let content = dialog.get_content_area();
        content.pack_start(&message, true, true, 0);

        dialog.set_default_size(200, -1);
        dialog.show_all();
        dialog.run();
        dialog.destroy();
    }
}