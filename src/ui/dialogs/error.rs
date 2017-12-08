use gtk::*;

pub fn error(msg: &str) {
    let dialog = Dialog::new();
    dialog.set_title("Systemd Manager: An Error Occurred");
    dialog.set_default_size(200, -1);

    let message = TextBuffer::new(None);
    message.set_text(msg);
    let message = TextView::new_with_buffer(&message);
    message.set_editable(false);
    message.set_wrap_mode(WrapMode::Word);
    message.set_border_width(5);

    let content = dialog.get_content_area();
    content.pack_start(&message, true, true, 0);
    dialog.show_all();
    dialog.run();
    dialog.destroy();
}