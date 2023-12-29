use futures::stream::StreamExt;
use r2r::QosProfile;

use std::sync::{Arc, Mutex};
use tokio::task;

const N_CONCURRENT_ROS_CONTEXT: usize = 3;
const N_TEARDOWN_CYCLES: usize = 2;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tokio_testing() -> Result<(), Box<dyn std::error::Error>> {

    let mut threads = futures::stream::FuturesUnordered::from_iter(
        (0..N_CONCURRENT_ROS_CONTEXT).map(|i_context| tokio::spawn(async move {
            // Iterate to check for memory corruption on node setup/teardown
            for i_cycle in 0..N_TEARDOWN_CYCLES {

                println!("tokio_testing iteration {i_cycle}");

                let ctx = r2r::Context::create().unwrap();
                // let ctx = std::thread::spawn(|| r2r::Context::create().unwrap()).join().unwrap();

                let mut node = r2r::Node::create(ctx, &format!("testnode_{i_context}"), "").unwrap();
                let mut s_the_no =
                    node.subscribe::<r2r::std_msgs::msg::Int32>(&format!("/the_no_{i_context}"), QosProfile::default()).unwrap();
                let mut s_new_no =
                    node.subscribe::<r2r::std_msgs::msg::Int32>(&format!("/new_no_{i_context}"), QosProfile::default()).unwrap();
                let p_the_no =
                    node.create_publisher::<r2r::std_msgs::msg::Int32>(&format!("/the_no_{i_context}"), QosProfile::default()).unwrap();
                let p_new_no =
                    node.create_publisher::<r2r::std_msgs::msg::Int32>(&format!("/new_no_{i_context}"), QosProfile::default()).unwrap();
                let state = Arc::new(Mutex::new(0));

                task::spawn(async move {
                    (0..10).for_each(|i| {
                        p_the_no
                            .publish(&r2r::std_msgs::msg::Int32 { data: i })
                            .unwrap();

                        println!("send {i}");

                    });
                });

                task::spawn(async move {
                    while let Some(msg) = s_the_no.next().await {
                        p_new_no
                            .publish(&r2r::std_msgs::msg::Int32 {
                                data: msg.data + 10,
                            })
                            .unwrap();

                        println!("got {}, send {}", msg.data, msg.data + 10);
                    }
                });

                let s = state.clone();
                task::spawn(async move {
                    while let Some(msg) = s_new_no.next().await {

                        println!("got {}", msg.data);

                        let i = msg.data;

                        *s.lock().unwrap() = i;
                    }
                });

                // std::thread::spawn doesn't work here anymore?
                let handle = task::spawn_blocking(move || {
                    for _ in 1..30 {
                        node.spin_once(std::time::Duration::from_millis(100));
                        let x = state.lock().unwrap();

                        println!("rec {}", x);

                        if *x == 19 {
                            break;
                        }
                    }

                    *state.lock().unwrap()
                });
                let x = handle.await.unwrap();
                assert_eq!(x, 19);

                println!("tokio_testing finish iteration {i_cycle}");

            }
    })));

    while let Some(thread) = threads.next().await {
        thread.unwrap();
    }

    Ok(())
}
