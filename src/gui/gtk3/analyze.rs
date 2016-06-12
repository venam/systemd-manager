use gtk::{Builder, CellRendererText, Label, ListStore, TreeView, TreeViewColumn, Type};
use systemd::analyze::Analyze;

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
pub fn setup(builder: &Builder) {
    if let Some(units) = Analyze::blame() {
        let analyze_tree: TreeView = builder.get_object("analyze_tree").unwrap();
        let analyze_store = ListStore::new(&[Type::U32, Type::String]);

        // A simple macro for adding a column to the preview tree.
        macro_rules! add_column {
            ($preview_tree:ident, $title:expr, $id:expr) => {{
                let column   = TreeViewColumn::new();
                let renderer = CellRendererText::new();
                column.set_title($title);
                column.set_resizable(true);
                column.pack_start(&renderer, true);
                column.add_attribute(&renderer, "text", $id);
                analyze_tree.append_column(&column);
            }}
        }

        add_column!(analyze_store, "Time (ms)", 0);
        add_column!(analyze_store, "Unit", 1);


        for value in units.clone() {
            analyze_store.insert_with_values(None, &[0, 1], &[&value.time, &value.service]);
        }

        analyze_tree.set_model(Some(&analyze_store));

        let kernel_time:    Label = builder.get_object("kernel_time_label").unwrap();
        let userspace_time: Label = builder.get_object("userspace_time_label").unwrap();
        let total_time:     Label = builder.get_object("total_time_label").unwrap();
        let times = Analyze::time();
        kernel_time.set_label(times.0.as_str());
        userspace_time.set_label(times.1.as_str());
        total_time.set_label(times.2.as_str());
    }
}
