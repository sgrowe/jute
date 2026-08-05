use crate::error::JuteResult;
use crate::parser::parse_tasks_file;
use crate::project_root::find_and_read_tasks_file;

pub fn list_tasks() -> JuteResult<()> {
    let tasks_file_raw = find_and_read_tasks_file()?;

    let tasks = parse_tasks_file(&tasks_file_raw)?;

    println!("Available tasks:");

    for task_name in tasks.list_tasks() {
        println!("- {task_name}");
    }

    Ok(())
}
