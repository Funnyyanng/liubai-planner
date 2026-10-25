use planner_contracts::*;
use planner_framework::{external::ExternalReader, UnitOfWork};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub id: u64,
    pub name: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub due: Option<Minute>,
    pub submitted: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub title: String,
    pub url: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub course: Course,
    pub assignments: Vec<Assignment>,
    pub events: Vec<CalendarEvent>,
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub undated: Vec<Assignment>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    pub course_id: u64,
    pub assignment_ids: Vec<u64>,
    pub minutes: u32,
    pub import_events: bool,
}
fn date(v: &Value) -> Option<Minute> {
    v.as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp() / 60)
}
fn title(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("未命名").to_owned()
}
pub fn courses(reader: &dyn ExternalReader) -> Result<Vec<Course>> {
    Ok(reader
        .list("/api/v1/courses?per_page=100")?
        .iter()
        .filter_map(|v| {
            Some(Course {
                id: v["id"].as_u64()?,
                name: v["name"].as_str()?.to_owned(),
            })
        })
        .collect())
}
pub fn fetch(reader: &dyn ExternalReader, course: Course, now: Minute) -> Result<Preview> {
    fetch_inner(reader, course, now, true)
}
fn fetch_inner(
    reader: &dyn ExternalReader,
    course: Course,
    now: Minute,
    files: bool,
) -> Result<Preview> {
    let mut warnings = vec![];
    let assignments = reader
        .list(&format!(
            "/api/v1/courses/{}/assignments?include[]=submission&per_page=100",
            course.id
        ))?
        .iter()
        .map(|v| {
            Ok(Assignment {
                id: v["id"]
                    .as_u64()
                    .ok_or_else(|| AppError::new("canvas_data", "作业缺少编号"))?,
                title: title(v, "name"),
                url: format!(
                    "https://oc.sjtu.edu.cn/courses/{}/assignments/{}",
                    course.id, v["id"]
                ),
                due: date(&v["due_at"]),
                submitted: date(&v["submission"]["submitted_at"])
                    .map(|t| t <= now)
                    .unwrap_or_else(|| {
                        matches!(
                            v["submission"]["workflow_state"].as_str(),
                            Some("submitted" | "graded" | "pending_review")
                        )
                    })
                    || v["submission"]["excused"].as_bool() == Some(true),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let stamp = |t| {
        chrono::DateTime::from_timestamp(t * 60, 0)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string()
    };
    let raw_events = reader.list(&format!("/api/v1/calendar_events?context_codes[]=course_{}&type=event&start_date={}&end_date={}&per_page=100",course.id,stamp(now-1440),stamp(now+60*1440)))?;
    let mut events = vec![];
    for v in raw_events {
        if let (Some(id), Some(start), Some(end)) =
            (v["id"].as_u64(), date(&v["start_at"]), date(&v["end_at"]))
        {
            if end > start && end > now {
                events.push(CalendarEvent {
                    id: format!("canvas:event:{id}"),
                    title: title(&v, "title"),
                    start,
                    end,
                    location: v["location_name"].as_str().unwrap_or("").into(),
                    equipment: vec![],
                    recurrence: None,
                    source: "canvas:oc.sjtu.edu.cn".into(),
                });
            } else if end <= start {
                warnings.push("有日历事件没有有效结束时间，未导入".into());
            }
        } else {
            warnings.push("有日历事件缺少起止时间，未导入".into());
        }
    }
    let resources = if !files {
        vec![]
    } else {
        match reader.list(&format!("/api/v1/courses/{}/files?per_page=100", course.id)) {
            Ok(files) => files
                .iter()
                .filter_map(|v| {
                    Some(Resource {
                        title: title(v, "display_name"),
                        url: format!(
                            "https://oc.sjtu.edu.cn/courses/{}/files/{}",
                            course.id,
                            v["id"].as_u64()?
                        ),
                    })
                })
                .collect(),
            Err(_) => {
                warnings.push("课程文件不可读取；作业与日历仍可同步".into());
                vec![]
            }
        }
    };
    Ok(Preview {
        course,
        assignments,
        events,
        resources,
        undated: vec![],
        warnings,
    })
}
/// Conservative filtering: undated work remains visible for review.
pub fn upcoming(mut p: Preview, now: Minute) -> Preview {
    let old = p.assignments.len();
    p.undated = p
        .assignments
        .iter()
        .filter(|a| !a.submitted && a.due.is_none())
        .cloned()
        .collect();
    p.assignments
        .retain(|a| !a.submitted && a.due.is_some_and(|d| d >= now));
    p.assignments.sort_by_key(|a| (a.due, a.id));
    p.events.retain(|e| e.end > now);
    let hidden = old - p.assignments.len() - p.undated.len();
    if hidden > 0 {
        p.warnings
            .push(format!("已隐藏 {hidden} 项已提交或截止时间已过的作业"));
    }
    p
}
pub fn scan(reader: &dyn ExternalReader, now: Minute) -> Result<Vec<Preview>> {
    let courses = courses(reader)?;
    let results = std::sync::Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(course) = courses.get(i) else {
                    break;
                };
                let p = match fetch_inner(reader, course.clone(), now, false) {
                    Ok(p) => upcoming(p, now),
                    Err(e) => Preview {
                        course: course.clone(),
                        assignments: vec![],
                        events: vec![],
                        resources: vec![],
                        undated: vec![],
                        warnings: vec![format!("未能检查该课程：{}", e.message)],
                    },
                };
                results.lock().unwrap().push(p);
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|p| {
        (
            p.assignments
                .first()
                .and_then(|a| a.due)
                .unwrap_or(i64::MAX),
            p.course.id,
        )
    });
    Ok(results)
}
pub fn cached(u: &UnitOfWork, course: u64) -> Result<Option<Preview>> {
    u.repo("canvas")?.get(&format!("course:{course}"))
}
pub fn save(u: &UnitOfWork, p: &Preview) -> Result<()> {
    u.repo("canvas")?.put(&format!("course:{}", p.course.id), p)
}
pub fn merge(p: &Preview, request: &Import, existing: &[Task]) -> Result<Vec<Task>> {
    if request.course_id != p.course.id || !(5..=10000).contains(&request.minutes) {
        return Err(AppError::new("canvas_input", "课程或估时无效"));
    }
    let mut result = vec![];
    let mut seen = std::collections::BTreeSet::new();
    for id in &request.assignment_ids {
        if !seen.insert(id) {
            continue;
        }
        let a = p
            .assignments
            .iter()
            .find(|a| a.id == *id)
            .ok_or_else(|| AppError::new("canvas_input", "选择的作业已不在预览中"))?;
        let deadline = a
            .due
            .ok_or_else(|| AppError::new("canvas_deadline", "无截止时间作业请手动创建任务"))?;
        let task_id = format!("canvas:assignment:{}:{}", p.course.id, a.id);
        let mut task = existing
            .iter()
            .find(|t| t.id == task_id)
            .cloned()
            .unwrap_or(Task {
                id: task_id,
                title: a.title.clone(),
                project: p.course.name.clone(),
                category: "课程学习".into(),
                remaining: request.minutes,
                estimate: request.minutes,
                deadline,
                priority: 2,
                min_chunk: request.minutes.min(25),
                splittable: true,
                conditions: Conditions::default(),
                dependencies: vec![],
                completed: false,
                blocked: false,
                completion_criteria: format!("阅读并完成 Canvas 作业要求：{}", a.url),
                deferrals: 0,
                parent_id: None,
            });
        task.deadline = deadline;
        result.push(task);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct MockReader;
    impl ExternalReader for MockReader {
        fn list(&self, path: &str) -> Result<Vec<Value>> {
            if path.contains("assignments?") {
                return Ok(vec![
                    serde_json::json!({"id":1,"name":"待交作业","due_at":"2026-05-06T00:00:00Z","submission":{"submitted_at":"2026-05-06T01:00:00Z","workflow_state":"submitted"}}),
                    serde_json::json!({"id":2,"name":"已交作业","due_at":"2026-05-06T00:00:00Z","submission":{"submitted_at":"2026-05-03T01:00:00Z","workflow_state":"submitted"}}),
                ]);
            }
            if path.contains("calendar_events?") {
                return Ok(vec![
                    serde_json::json!({"id":1,"title":"旧课","start_at":"2026-05-03T00:00:00Z","end_at":"2026-05-03T01:00:00Z"}),
                    serde_json::json!({"id":2,"title":"未来课","start_at":"2026-05-06T00:00:00Z","end_at":"2026-05-06T01:00:00Z"}),
                ]);
            }
            Ok(vec![])
        }
    }
    #[test]
    fn reference_date_applies_to_submission_and_calendar() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-05T00:00:00+08:00")
            .unwrap()
            .timestamp()
            / 60;
        let p = upcoming(
            fetch(
                &MockReader,
                Course {
                    id: 1,
                    name: "课".into(),
                },
                now,
            )
            .unwrap(),
            now,
        );
        assert_eq!(p.assignments.len(), 1);
        assert_eq!(p.assignments[0].id, 1);
        assert_eq!(p.events.len(), 1);
        assert_eq!(p.events[0].id, "canvas:event:2");
    }
    #[test]
    fn filters_future_work_without_losing_undated_items() {
        let a = |id, due, submitted| Assignment {
            id,
            title: "任务".into(),
            url: String::new(),
            due,
            submitted,
        };
        let p = Preview {
            course: Course {
                id: 1,
                name: "课程".into(),
            },
            assignments: vec![
                a(1, Some(99), false),
                a(2, Some(100), false),
                a(3, Some(101), true),
                a(4, None, false),
            ],
            events: vec![],
            resources: vec![],
            undated: vec![],
            warnings: vec![],
        };
        let p = upcoming(p, 100);
        assert_eq!(p.assignments.len(), 1);
        assert_eq!(p.assignments[0].id, 2);
        assert_eq!(p.undated.len(), 1);
    }
    #[test]
    fn repeated_sync_preserves_progress_and_local_fields() {
        let mut p = Preview {
            course: Course {
                id: 1,
                name: "课程".into(),
            },
            assignments: vec![Assignment {
                id: 2,
                title: "作业".into(),
                url: "https://oc.sjtu.edu.cn/".into(),
                due: Some(100),
                submitted: true,
            }],
            events: vec![],
            resources: vec![],
            undated: vec![],
            warnings: vec![],
        };
        let r = Import {
            course_id: 1,
            assignment_ids: vec![2, 2],
            minutes: 60,
            import_events: false,
        };
        let mut old = merge(&p, &r, &[]).unwrap();
        assert_eq!(old.len(), 1);
        assert!(!old[0].completed);
        old[0].remaining = 15;
        old[0].title = "我的标题".into();
        old[0].dependencies = vec!["other".into()];
        p.assignments[0].due = Some(200);
        let new = merge(&p, &r, &old).unwrap();
        assert_eq!(new[0].remaining, 15);
        assert_eq!(new[0].title, "我的标题");
        assert_eq!(new[0].deadline, 200);
        assert_eq!(new[0].dependencies, vec!["other"]);
        p.assignments[0].due = None;
        assert!(merge(&p, &r, &old).is_err());
    }
}
