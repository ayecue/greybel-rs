/// A police and a student record.
pub const DATABASE: &str = concat!(
    r#"{"records":[{"kind":2,"person_id":"7","full_name":"Jane Doe","age":31,"phone":"512345678","#,
    r#""mail_address":"doe@police.test","face_parts":[{"name":"hair3","scale":{"x":1.5,"y":1.0},"#,
    r#""position":{"x":0.25,"y":-1.0}}],"schedule":"","license_plate":"","local_ip":"","work_performance":0,"#,
    r#""job_role":0,"late_work_strikes":0,"dead":false,"fired":false,"#,
    r#""offenses":[{"id":0,"crime":8,"start_year":1991}]},"#,
    r#"{"kind":1,"person_id":"8","full_name":"John Roe","age":20,"phone":"","mail_address":"","face_parts":[],"#,
    r#""schedule":"","license_plate":"","local_ip":"","work_performance":0,"job_role":0,"late_work_strikes":0,"#,
    r#""dead":false,"fired":false,"grades":[{"subject":2,"mark":4.5}]}]}"#,
);
