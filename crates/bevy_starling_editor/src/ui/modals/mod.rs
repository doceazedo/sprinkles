mod confirm_delete;
mod new_project;

#[allow(unused_imports)]
pub use confirm_delete::{draw_confirm_delete_modal, ConfirmDeleteModal};
#[allow(unused_imports)]
pub use new_project::{
    draw_new_project_modal, on_create_project_event, CreateProjectEvent, NewProjectModal,
};
