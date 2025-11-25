use aetos::core::PrometheusMetric;
use aetos::{Label, define_histogram, metrics};

#[test]
fn test_scalar_counter() {
    #[metrics]
    struct TestMetrics {
        #[counter(help = "Total requests")]
        requests: u64,
    }

    let m = TestMetrics { requests: 42 };
    let output = m.to_string();

    assert!(output.contains("# HELP requests Total requests\n"));
    assert!(output.contains("# TYPE requests counter\n"));
    assert!(output.contains("requests 42\n"));
}

#[test]
fn test_scalar_gauge() {
    #[metrics]
    struct TestMetrics {
        #[gauge(help = "Current temperature")]
        temperature: f64,
    }

    let m = TestMetrics { temperature: 23.5 };
    let output = m.to_string();

    assert!(output.contains("# HELP temperature Current temperature\n"));
    assert!(output.contains("# TYPE temperature gauge\n"));
    assert!(output.contains("temperature 23.5\n"));
}

#[test]
fn test_with_prefix() {
    #[metrics(prefix = "myapp")]
    struct TestMetrics {
        #[counter(help = "Test counter")]
        count: u64,
    }

    let m = TestMetrics { count: 100 };
    let output = m.to_string();

    assert!(output.contains("# HELP myapp_count Test counter\n"));
    assert!(output.contains("# TYPE myapp_count counter\n"));
    assert!(output.contains("myapp_count 100\n"));
}

#[test]
fn test_name_override() {
    #[metrics]
    struct TestMetrics {
        #[counter(help = "Custom name", name = "custom_metric_name")]
        field: u64,
    }

    let m = TestMetrics { field: 77 };
    let output = m.to_string();

    assert!(output.contains("# HELP custom_metric_name Custom name\n"));
    assert!(output.contains("# TYPE custom_metric_name counter\n"));
    assert!(output.contains("custom_metric_name 77\n"));
}

#[test]
fn test_single_label_with_explicit_name() {
    #[metrics]
    struct TestMetrics {
        #[counter(help = "Events by type", label = "event_type")]
        events: Vec<(String, u64)>,
    }

    let m = TestMetrics {
        events: vec![("stake".to_string(), 10), ("unstake".to_string(), 5)],
    };
    let output = m.to_string();

    assert!(output.contains("# HELP events Events by type\n"));
    assert!(output.contains("# TYPE events counter\n"));
    assert!(output.contains(r#"events{event_type="stake"} 10"#));
    assert!(output.contains(r#"events{event_type="unstake"} 5"#));
}

#[test]
fn test_single_label_derived_name() {
    #[derive(Debug)]
    enum HttpMethod {
        Get,
        Post,
    }

    impl std::fmt::Display for HttpMethod {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[metrics]
    struct TestMetrics {
        #[counter(help = "Requests by method", label = "http_method")]
        requests: Vec<(HttpMethod, u64)>,
    }

    let m = TestMetrics {
        requests: vec![(HttpMethod::Get, 100), (HttpMethod::Post, 50)],
    };
    let output = m.to_string();

    assert!(output.contains("# HELP requests Requests by method\n"));
    assert!(output.contains("# TYPE requests counter\n"));
    assert!(output.contains(r#"requests{http_method="Get"} 100"#));
    assert!(output.contains(r#"requests{http_method="Post"} 50"#));
}

#[test]
fn test_multi_label() {
    #[derive(Label)]
    struct RequestLabels {
        method: String,
        status: u32,
    }

    #[metrics]
    struct TestMetrics {
        #[counter(help = "HTTP requests")]
        http_requests: Vec<(RequestLabels, u64)>,
    }

    let m = TestMetrics {
        http_requests: vec![
            (
                RequestLabels {
                    method: "GET".to_string(),
                    status: 200,
                },
                150,
            ),
            (
                RequestLabels {
                    method: "POST".to_string(),
                    status: 404,
                },
                3,
            ),
        ],
    };
    let output = m.to_string();

    assert!(output.contains("# HELP http_requests HTTP requests\n"));
    assert!(output.contains("# TYPE http_requests counter\n"));
    assert!(output.contains(r#"http_requests{method="GET",status="200"} 150"#));
    assert!(output.contains(r#"http_requests{method="POST",status="404"} 3"#));
}

#[test]
fn test_mixed_fields() {
    #[derive(Label)]
    struct Labels {
        region: String,
    }

    #[metrics(prefix = "app")]
    struct TestMetrics {
        #[counter(help = "Total count")]
        total: u64,

        #[gauge(help = "Current value")]
        current: f64,

        #[counter(help = "Events by type", label = "type")]
        events: Vec<(String, u64)>,

        #[counter(help = "Requests by region")]
        requests: Vec<(Labels, u64)>,
    }

    let m = TestMetrics {
        total: 1000,
        current: 42.5,
        events: vec![("error".to_string(), 5)],
        requests: vec![(
            Labels {
                region: "us-east".to_string(),
            },
            200,
        )],
    };

    let output = m.to_string();

    assert!(output.contains("# HELP app_total Total count\n"));
    assert!(output.contains("# TYPE app_total counter\n"));
    assert!(output.contains("app_total 1000\n"));

    assert!(output.contains("# HELP app_current Current value\n"));
    assert!(output.contains("# TYPE app_current gauge\n"));
    assert!(output.contains("app_current 42.5\n"));

    assert!(output.contains("# HELP app_events Events by type\n"));
    assert!(output.contains(r#"app_events{type="error"} 5"#));

    assert!(output.contains("# HELP app_requests Requests by region\n"));
    assert!(output.contains(r#"app_requests{region="us-east"} 200"#));
}

#[test]
#[cfg(not(feature = "no-escaping"))]
fn test_label_value_escaping_in_metrics() {
    #[metrics]
    struct TestMetrics {
        #[counter(help = "Test", label = "value")]
        test: Vec<(String, u64)>,
    }

    let m = TestMetrics {
        test: vec![(r#"has"quotes"#.to_string(), 1)],
    };

    let output = m.to_string();
    assert!(output.contains(r#"test{value="has\"quotes"} 1"#));
}

#[test]
fn test_prometheus_metric_trait() {
    #[metrics]
    struct TestMetrics {
        #[counter(help = "Test")]
        count: u64,
    }

    let m = TestMetrics { count: 42 };

    fn accepts_prometheus_metric<T: PrometheusMetric>(_: &T) {}
    accepts_prometheus_metric(&m);
}

#[test]
fn test_borrowed_str_labels() {
    #[metrics]
    struct TestMetrics<'a> {
        #[counter(help = "Requests by endpoint", label = "endpoint")]
        requests_by_endpoint: Vec<(&'a str, u64)>,
    }

    let m = TestMetrics {
        requests_by_endpoint: vec![("/api/users", 100), ("/api/posts", 50), ("/health", 25)],
    };

    let output = m.to_string();

    assert!(output.contains("# HELP requests_by_endpoint Requests by endpoint\n"));
    assert!(output.contains("# TYPE requests_by_endpoint counter\n"));
    assert!(output.contains(r#"requests_by_endpoint{endpoint="/api/users"} 100"#));
    assert!(output.contains(r#"requests_by_endpoint{endpoint="/api/posts"} 50"#));
    assert!(output.contains(r#"requests_by_endpoint{endpoint="/health"} 25"#));
}

#[test]
fn test_hashmap_single_label() {
    use std::collections::HashMap;

    #[metrics]
    struct TestMetrics {
        #[counter(help = "Requests by method", label = "method")]
        requests: HashMap<String, u64>,
    }

    let mut requests = HashMap::new();
    requests.insert("GET".to_string(), 100);
    requests.insert("POST".to_string(), 50);

    let m = TestMetrics { requests };
    let output = m.to_string();

    assert!(output.contains("# HELP requests Requests by method\n"));
    assert!(output.contains("# TYPE requests counter\n"));
    assert!(output.contains(r#"requests{method="GET"} 100"#));
    assert!(output.contains(r#"requests{method="POST"} 50"#));
}

#[test]
fn test_hashmap_multi_label() {
    use std::collections::HashMap;

    #[derive(Label, Clone, PartialEq, Eq, Hash)]
    struct HttpLabels {
        method: String,
        status: u16,
    }

    #[metrics]
    struct TestMetrics {
        #[counter(help = "HTTP requests")]
        http_requests: HashMap<HttpLabels, u64>,
    }

    let mut http_requests = HashMap::new();
    http_requests.insert(
        HttpLabels {
            method: "GET".to_string(),
            status: 200,
        },
        150,
    );
    http_requests.insert(
        HttpLabels {
            method: "POST".to_string(),
            status: 404,
        },
        3,
    );

    let m = TestMetrics { http_requests };
    let output = m.to_string();

    assert!(output.contains("# HELP http_requests HTTP requests\n"));
    assert!(output.contains("# TYPE http_requests counter\n"));
    assert!(output.contains(r#"http_requests{method="GET",status="200"} 150"#));
    assert!(output.contains(r#"http_requests{method="POST",status="404"} 3"#));
}

#[test]
fn test_btreemap_single_label() {
    use std::collections::BTreeMap;

    #[metrics]
    struct TestMetrics {
        #[counter(help = "Events by type", label = "event_type")]
        events: BTreeMap<String, u64>,
    }

    let mut events = BTreeMap::new();
    events.insert("stake".to_string(), 10);
    events.insert("unstake".to_string(), 5);

    let m = TestMetrics { events };
    let output = m.to_string();

    assert!(output.contains("# HELP events Events by type\n"));
    assert!(output.contains("# TYPE events counter\n"));
    assert!(output.contains(r#"events{event_type="stake"} 10"#));
    assert!(output.contains(r#"events{event_type="unstake"} 5"#));
}

#[test]
fn test_btreemap_multi_label() {
    use std::collections::BTreeMap;

    #[derive(Label, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct MetricLabels {
        region: String,
        zone: String,
    }

    #[metrics]
    struct TestMetrics {
        #[counter(help = "Requests by region and zone")]
        requests: BTreeMap<MetricLabels, u64>,
    }

    let mut requests = BTreeMap::new();
    requests.insert(
        MetricLabels {
            region: "us-east".to_string(),
            zone: "1a".to_string(),
        },
        200,
    );
    requests.insert(
        MetricLabels {
            region: "us-west".to_string(),
            zone: "2b".to_string(),
        },
        150,
    );

    let m = TestMetrics { requests };
    let output = m.to_string();

    assert!(output.contains("# HELP requests Requests by region and zone\n"));
    assert!(output.contains("# TYPE requests counter\n"));
    assert!(output.contains(r#"requests{region="us-east",zone="1a"} 200"#));
    assert!(output.contains(r#"requests{region="us-west",zone="2b"} 150"#));
}

#[test]
fn test_histogram_with_labels() {
    #[derive(Label, Hash, Eq, PartialEq, Clone, Debug)]
    struct RequestLabel {
        method: &'static str,
        status: u16,
    }

    define_histogram!(RequestLatency<RequestLabel> = [0.1, 0.5, 1.0]);

    #[metrics]
    struct TestMetrics {
        #[histogram(help = "Request latency in seconds")]
        latency: RequestLatency,
    }

    let mut m = TestMetrics {
        latency: RequestLatency::default(),
    };

    m.latency.observe(
        RequestLabel {
            method: "GET",
            status: 200,
        },
        0.25,
    );
    m.latency.observe(
        RequestLabel {
            method: "GET",
            status: 200,
        },
        0.75,
    );
    m.latency.observe(
        RequestLabel {
            method: "POST",
            status: 201,
        },
        0.15,
    );

    let output = m.to_string();

    assert!(output.contains("# HELP latency Request latency in seconds\n"));
    assert!(output.contains("# TYPE latency histogram\n"));

    // GET 200 observations (0.25 and 0.75)
    assert!(output.contains(r#"latency_bucket{method="GET",status="200",le="0.1"} 0"#));
    assert!(output.contains(r#"latency_bucket{method="GET",status="200",le="0.5"} 1"#));
    assert!(output.contains(r#"latency_bucket{method="GET",status="200",le="1"} 2"#));
    assert!(output.contains(r#"latency_bucket{method="GET",status="200",le="+Inf"} 2"#));
    assert!(output.contains(r#"latency_sum{method="GET",status="200"} 1"#));
    assert!(output.contains(r#"latency_count{method="GET",status="200"} 2"#));

    // POST 201 observation (0.15)
    assert!(output.contains(r#"latency_bucket{method="POST",status="201",le="0.1"} 0"#));
    assert!(output.contains(r#"latency_bucket{method="POST",status="201",le="0.5"} 1"#));
    assert!(output.contains(r#"latency_bucket{method="POST",status="201",le="1"} 1"#));
    assert!(output.contains(r#"latency_bucket{method="POST",status="201",le="+Inf"} 1"#));
    assert!(output.contains(r#"latency_sum{method="POST",status="201"} 0.15"#));
    assert!(output.contains(r#"latency_count{method="POST",status="201"} 1"#));
}

#[test]
fn test_histogram_unlabeled() {
    define_histogram!(ResponseTime<()> = [0.05, 0.1, 0.5]);

    #[metrics]
    struct TestMetrics {
        #[histogram(help = "Response time distribution")]
        response_time: ResponseTime,
    }

    let mut m = TestMetrics {
        response_time: ResponseTime::default(),
    };

    m.response_time.observe((), 0.03);
    m.response_time.observe((), 0.08);
    m.response_time.observe((), 0.45);

    let output = m.to_string();

    assert!(output.contains("# HELP response_time Response time distribution\n"));
    assert!(output.contains("# TYPE response_time histogram\n"));

    // Check bucket counts (cumulative)
    assert!(output.contains(r#"response_time_bucket{le="0.05"} 1"#));
    assert!(output.contains(r#"response_time_bucket{le="0.1"} 2"#));
    assert!(output.contains(r#"response_time_bucket{le="0.5"} 3"#));
    assert!(output.contains(r#"response_time_bucket{le="+Inf"} 3"#));

    // Check sum and count
    assert!(output.contains(r#"response_time_sum{} 0.56"#));
    assert!(output.contains(r#"response_time_count{} 3"#));
}
