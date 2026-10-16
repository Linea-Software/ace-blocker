use crate::common::get_default_test_server;
use ace_engine::service::api::schemas::*;
use chrono::NaiveTime;
use fake::{Fake, Faker};
use rand::RngExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_add() {
    let test_server = get_default_test_server();

    // Create Network List
    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: true,
            sunday: true,
        }],
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    assert_eq!(body.schedule_entries.len(), 1);

    assert!(body.schedule_entries.iter().any(|sched| sched.start_time
        == NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        && sched.end_time == NaiveTime::from_hms_opt(10, 0, 0).unwrap()
        && sched.monday
        && sched.tuesday
        && sched.wednesday
        && sched.thursday
        && sched.friday
        && sched.saturday
        && sched.sunday));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_merging() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_merge_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // Add two overlapping entries
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
        ],
    };
    let res = test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    // Should be merged into one entry 10:00 - 13:00
    assert_eq!(body.schedule_entries.len(), 1);
    let entry = &body.schedule_entries[0];
    assert_eq!(entry.start_time, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
    assert_eq!(entry.end_time, NaiveTime::from_hms_opt(13, 0, 0).unwrap());
    assert!(entry.monday);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_removal() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_remove_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            monday: true,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
        }],
    };
    let _ = test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    assert_eq!(body.schedule_entries.len(), 1);
    let entry_id = body.schedule_entries[0].id;

    let res = test_server.delete(&format!("/schedule/{}", entry_id)).await;
    res.assert_status_ok();

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    assert_eq!(body.schedule_entries.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_complex_scenarios() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_complex_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // 1. Add a base interval
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            monday: true,
            tuesday: true,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
        }],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    // 2. Add a subset interval (should be ignored/merged)
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(11, 30, 0).unwrap(),
            monday: true,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
        }],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    assert_eq!(body.schedule_entries.len(), 1);
    let entry = &body.schedule_entries[0];
    assert_eq!(entry.start_time, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
    assert_eq!(entry.end_time, NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    assert!(entry.monday);
    assert!(entry.tuesday);

    // 3. Add an interval for all days
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            monday: true,
            tuesday: true,
            wednesday: true,
            thursday: true,
            friday: true,
            saturday: true,
            sunday: true,
        }],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    assert_eq!(body.schedule_entries.len(), 2);

    let all_days_entry = body
        .schedule_entries
        .iter()
        .find(|e| e.start_time == NaiveTime::from_hms_opt(14, 0, 0).unwrap())
        .expect("Should find 14:00 entry");

    assert!(all_days_entry.monday);
    assert!(all_days_entry.sunday);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_brutal_gaps() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_brutal_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // 1. Add multiple unmergeable schedules (disjoint)
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
        ],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    // 2. Add schedules that fill some gaps but not all
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(13, 30, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
            BaseScheduleEntrySchema {
                start_time: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                monday: true,
                tuesday: false,
                wednesday: false,
                thursday: false,
                friday: false,
                saturday: false,
                sunday: false,
            },
        ],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    assert_eq!(body.schedule_entries.len(), 3);

    let mut entries = body.schedule_entries.clone();
    entries.sort_by_key(|a| a.start_time);

    assert_eq!(
        entries[0].start_time,
        NaiveTime::from_hms_opt(10, 0, 0).unwrap()
    );
    assert_eq!(
        entries[0].end_time,
        NaiveTime::from_hms_opt(13, 30, 0).unwrap()
    );

    assert_eq!(
        entries[1].start_time,
        NaiveTime::from_hms_opt(14, 0, 0).unwrap()
    );
    assert_eq!(
        entries[1].end_time,
        NaiveTime::from_hms_opt(15, 0, 0).unwrap()
    );

    assert_eq!(
        entries[2].start_time,
        NaiveTime::from_hms_opt(15, 30, 0).unwrap()
    );
    assert_eq!(
        entries[2].end_time,
        NaiveTime::from_hms_opt(17, 0, 0).unwrap()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_time_coverage() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_coverage_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // Create 24 entries covering every hour
    let mut entries = Vec::new();
    for hour in 0..24 {
        let start = NaiveTime::from_hms_opt(hour, 0, 0).unwrap();
        // Handle wrapping for 23-00 which technically is handled by the manager as 00:00 next day
        let end = if hour == 23 {
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        } else {
            NaiveTime::from_hms_opt(hour + 1, 0, 0).unwrap()
        };

        entries.push(BaseScheduleEntrySchema {
            start_time: start,
            end_time: end,
            monday: true,
            tuesday: false, // Only Monday to simplify checking merge
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
        });
    }

    let req = AddScheduleEntriesRequest {
        schedule_entries: entries,
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;

    // Verify
    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    // Should merge into ONE entry 00:00 -> 00:00 (whole day)
    assert_eq!(
        body.schedule_entries.len(),
        1,
        "Should merge all adjacent hourly entries into one"
    );
    let entry = &body.schedule_entries[0];
    assert_eq!(entry.start_time, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    assert_eq!(entry.end_time, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    assert!(entry.monday);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_fuzz_random() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_fuzz_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // Generate random entries using Fake/Rand
    let mut rng = fake::rand::rng();
    let mut entries = Vec::new();

    // Create 10 random distinct intervals
    for _ in 0..10 {
        let h_start = rng.random_range(0..20);
        let h_duration = rng.random_range(1..4);
        let start = NaiveTime::from_hms_opt(h_start, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(h_start + h_duration, 0, 0).unwrap();

        entries.push(BaseScheduleEntrySchema {
            start_time: start,
            end_time: end,
            monday: Faker.fake(),
            tuesday: Faker.fake(),
            wednesday: Faker.fake(),
            thursday: Faker.fake(),
            friday: Faker.fake(),
            saturday: Faker.fake(),
            sunday: Faker.fake(),
        });
    }

    let req = AddScheduleEntriesRequest {
        schedule_entries: entries.clone(),
    };

    // Should succeed
    let res = test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await;
    res.assert_status_ok();

    // Verify they exist (might be fewer due to merge)
    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();

    // Assert that we get a valid response and the count is reasonable
    assert!(
        body.schedule_entries.len() <= 100,
        "Too many entries after merge: {}",
        body.schedule_entries.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_schedule_lock_constraints() {
    let test_server = get_default_test_server();

    let block_name: String = Faker.fake();
    let req = CreateNetworkBlockRequest {
        block_name: format!("test_lock_{}", block_name),
        icon: None,
    };
    let res = test_server.post("/network-blocks").json(&req).await;
    let body: CreateNetworkBlockResponse = res.json();
    let net_list_id = body.network_block.id;

    // 1. Add an initial entry
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            monday: true,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
        }],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await
        .assert_status_ok();

    // 2. Lock the block (ACTIVE_UNTIL_LOCK_END — within the 90-day cap)
    let far_future = (chrono::Local::now() + chrono::Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();

    let req = SetNetworkLockRequest {
        lock_state: "active_until_lock_end".to_string(),
        lock_end: Some(far_future),
    };
    test_server
        .post(&format!("/network-blocks/{}/lock", net_list_id))
        .json(&req)
        .await
        .assert_status_ok();

    // 3. Adding schedule entries should SUCCEED (Expansion/Stricter)
    let req = AddScheduleEntriesRequest {
        schedule_entries: vec![BaseScheduleEntrySchema {
            start_time: NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            monday: true,
            tuesday: false,
            wednesday: false,
            thursday: false,
            friday: false,
            saturday: false,
            sunday: false,
        }],
    };
    test_server
        .post(&format!("/network-blocks/{}/schedule", net_list_id))
        .json(&req)
        .await
        .assert_status_ok();

    // 4. Deleting schedule entry should FAIL (Reduction)
    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();
    // The schedule's entry order is not guaranteed by the API; find the
    // 10:00–11:00 entry explicitly.
    let entry_id = body
        .schedule_entries
        .iter()
        .find(|e| e.start_time == NaiveTime::from_hms_opt(10, 0, 0).unwrap())
        .expect("the 10:00 schedule entry must exist")
        .id;

    let res = test_server.delete(&format!("/schedule/{}", entry_id)).await;
    res.assert_status(http::StatusCode::FORBIDDEN);

    // 5. Editing schedule entry to be MORE restrictive should SUCCEED
    let edit_req = BaseScheduleEntrySchema {
        start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        end_time: NaiveTime::from_hms_opt(12, 0, 0).unwrap(), // Extension (more restrictive)
        monday: true,
        tuesday: false,
        wednesday: false,
        thursday: false,
        friday: false,
        saturday: false,
        sunday: false,
    };
    let res = test_server
        .put(&format!("/schedule/{}", entry_id))
        .json(&edit_req)
        .await;
    res.assert_status_ok();

    // 6. Editing to be LESS restrictive should FAIL
    // Get the updated entry ID after reprocessing
    let res = test_server
        .get(&format!("/network-blocks/{}/schedule", net_list_id))
        .await;
    let body: GetScheduleEntriesResponse = res.json();
    let updated_entry = body
        .schedule_entries
        .iter()
        .find(|e| {
            e.start_time == NaiveTime::from_hms_opt(10, 0, 0).unwrap()
                && e.end_time == NaiveTime::from_hms_opt(12, 0, 0).unwrap()
        })
        .expect("Should find the updated entry");

    let edit_req_reduce = BaseScheduleEntrySchema {
        start_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        end_time: NaiveTime::from_hms_opt(10, 30, 0).unwrap(), // Reduction (less restrictive)
        monday: true,
        tuesday: false,
        wednesday: false,
        thursday: false,
        friday: false,
        saturday: false,
        sunday: false,
    };
    let res = test_server
        .put(&format!("/schedule/{}", updated_entry.id))
        .json(&edit_req_reduce)
        .await;
    res.assert_status(http::StatusCode::FORBIDDEN);
}
