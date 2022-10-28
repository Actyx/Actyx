use std::future::Future;
use std::io::Write;
use std::time::Duration;

use crate::{internal_app_id, BanyanStore};
use actyx_sdk::{tags, Payload, StreamNr};
use anyhow::Result;
use libipld::cbor::cbor::MajorKind;
use libipld::cbor::encode::{write_u64, write_u8};
use libipld::cbor::DagCborCodec;
use libipld::codec::Encode;
use libipld::DagCbor;
use prometheus::{Encoder, Registry};

pub fn metrics(store: BanyanStore, nr: StreamNr, interval: Duration) -> Result<impl Future<Output = ()>> {
    let registry = Registry::new();
    store.ipfs().register_metrics(&registry)?;
    let tags = tags!("metrics");

    Ok(async move {
        let encoder = CborEncoder::new();
        let mut buffer = vec![];
        loop {
            tokio::time::sleep(interval).await;
            let mf = registry.gather();
            buffer.clear();
            if let Err(err) = encoder.encode(&mf, &mut buffer) {
                tracing::warn!("error encoding metrics: {}", err);
                continue;
            }
            if let Err(err) = store
                .append(
                    nr,
                    internal_app_id(),
                    vec![(tags.clone(), Payload::from_slice(&buffer))],
                )
                .await
            {
                tracing::warn!("error appending metrics: {}", err);
            }
        }
    })
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct MetricFamily {
    pub name: String,
    pub help: String,
    pub metrics: Vec<Metric>,
}

#[derive(Clone, DagCbor, Debug, PartialEq)]
#[ipld(repr = "int-tuple")]
pub enum Metric {
    #[ipld(repr = "value")]
    Counter(Counter),
    #[ipld(repr = "value")]
    Gauge(Gauge),
    #[ipld(repr = "value")]
    Summary(Summary),
    #[ipld(repr = "value")]
    Untyped(Untyped),
    #[ipld(repr = "value")]
    Histogram(Histogram),
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Counter {
    pub labels: Vec<LabelPair>,
    pub timestamp_ms: u64,
    pub value: f64,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Gauge {
    pub labels: Vec<LabelPair>,
    pub timestamp_ms: u64,
    pub value: f64,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Summary {
    pub labels: Vec<LabelPair>,
    pub timestamp_ms: u64,
    pub sample_count: u64,
    pub sample_sum: f64,
    pub quantiles: Vec<Quantile>,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Untyped {
    pub labels: Vec<LabelPair>,
    pub timestamp_ms: u64,
    pub value: f64,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Histogram {
    pub labels: Vec<LabelPair>,
    pub timestamp_ms: u64,
    pub sample_count: u64,
    pub sample_sum: f64,
    pub buckets: Vec<Bucket>,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct Bucket {
    pub cumulative_count: u64,
    pub upper_bound: f64,
}

#[derive(Clone, Debug, DagCbor, PartialEq)]
#[ipld(repr = "tuple")]
pub struct LabelPair {
    pub name: String,
    pub value: String,
}

#[derive(Default)]
pub struct CborEncoder {}

impl CborEncoder {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Encoder for CborEncoder {
    fn format_type(&self) -> &str {
        "cbor"
    }

    fn encode<W: Write>(&self, families: &[prometheus::proto::MetricFamily], w: &mut W) -> prometheus::Result<()> {
        prometheus_encode(families, w).map_err(|err| prometheus::Error::Msg(err.to_string()))
    }
}

fn prometheus_encode<W: Write>(families: &[prometheus::proto::MetricFamily], w: &mut W) -> Result<()> {
    let c = DagCborCodec;
    write_u64(w, MajorKind::Array, families.len() as u64)?;
    for family in families {
        write_u8(w, MajorKind::Array, 3)?;
        family.get_name().encode(c, w)?;
        family.get_help().encode(c, w)?;
        let ty = family.get_field_type() as u8;
        let len = match family.get_field_type() {
            prometheus::proto::MetricType::COUNTER => 3,
            prometheus::proto::MetricType::GAUGE => 3,
            prometheus::proto::MetricType::SUMMARY => 5,
            prometheus::proto::MetricType::UNTYPED => 3,
            prometheus::proto::MetricType::HISTOGRAM => 5,
        };
        write_u64(w, MajorKind::Array, family.get_metric().len() as u64)?;
        for metric in family.get_metric() {
            write_u8(w, MajorKind::Array, 2)?;
            ty.encode(c, w)?;
            write_u8(w, MajorKind::Array, len)?;
            let labels = metric.get_label();
            write_u64(w, MajorKind::Array, labels.len() as u64)?;
            for label in labels {
                write_u8(w, MajorKind::Array, 2)?;
                label.get_name().encode(c, w)?;
                label.get_value().encode(c, w)?;
            }
            (metric.get_timestamp_ms() as u64).encode(c, w)?;
            if metric.has_counter() {
                let counter = metric.get_counter();
                counter.get_value().encode(c, w)?;
            }
            if metric.has_gauge() {
                let gauge = metric.get_gauge();
                gauge.get_value().encode(c, w)?;
            }
            if metric.has_summary() {
                let summary = metric.get_summary();
                summary.get_sample_count().encode(c, w)?;
                summary.get_sample_sum().encode(c, w)?;
                let quantiles = summary.get_quantile();
                write_u64(w, MajorKind::Array, quantiles.len() as u64)?;
                for quantile in quantiles {
                    write_u8(w, MajorKind::Array, 2)?;
                    quantile.get_quantile().encode(c, w)?;
                    quantile.get_value().encode(c, w)?;
                }
            }
            if metric.has_untyped() {
                let untyped = metric.get_untyped();
                untyped.get_value().encode(c, w)?;
            }
            if metric.has_histogram() {
                let histogram = metric.get_histogram();
                histogram.get_sample_count().encode(c, w)?;
                histogram.get_sample_sum().encode(c, w)?;
                let buckets = histogram.get_bucket();
                write_u64(w, MajorKind::Array, buckets.len() as u64)?;
                for bucket in buckets {
                    write_u8(w, MajorKind::Array, 2)?;
                    bucket.get_cumulative_count().encode(c, w)?;
                    bucket.get_upper_bound().encode(c, w)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use libipld::cbor::DagCborCodec;
    use libipld::codec::Codec;
    use prometheus::core::Collector;

    #[test]
    fn test_cbor_encoder() -> Result<()> {
        let counter_opts = prometheus::Opts::new("test_counter", "test help")
            .const_label("a", "1")
            .const_label("b", "2");
        let counter = prometheus::Counter::with_opts(counter_opts).unwrap();
        counter.inc();
        let mf = counter.collect();
        let mut buffer = vec![];
        let encoder = CborEncoder::new();
        encoder.encode(&mf, &mut buffer)?;
        let mf: Vec<MetricFamily> = DagCborCodec.decode(&buffer)?;
        assert_eq!(
            mf,
            vec![MetricFamily {
                name: "test_counter".into(),
                help: "test help".into(),
                metrics: vec![Metric::Counter(Counter {
                    labels: vec![
                        LabelPair {
                            name: "a".into(),
                            value: "1".into()
                        },
                        LabelPair {
                            name: "b".into(),
                            value: "2".into()
                        }
                    ],
                    timestamp_ms: 0,
                    value: 1.0,
                })]
            }]
        );

        let gauge_opts = prometheus::Opts::new("test_gauge", "test help")
            .const_label("a", "1")
            .const_label("b", "2");
        let gauge = prometheus::Gauge::with_opts(gauge_opts).unwrap();
        gauge.inc();
        gauge.set(42.0);
        let mf = gauge.collect();
        buffer.clear();
        encoder.encode(&mf, &mut buffer)?;
        let mf: Vec<MetricFamily> = DagCborCodec.decode(&buffer)?;
        assert_eq!(
            mf,
            vec![MetricFamily {
                name: "test_gauge".into(),
                help: "test help".into(),
                metrics: vec![Metric::Gauge(Gauge {
                    labels: vec![
                        LabelPair {
                            name: "a".into(),
                            value: "1".into()
                        },
                        LabelPair {
                            name: "b".into(),
                            value: "2".into()
                        }
                    ],
                    timestamp_ms: 0,
                    value: 42.0,
                })]
            }]
        );
        Ok(())
    }

    #[test]
    fn test_cbor_encoder_histogram() -> Result<()> {
        let opts = prometheus::HistogramOpts::new("test_histogram", "test help").const_label("a", "1");
        let histogram = prometheus::Histogram::with_opts(opts).unwrap();
        histogram.observe(0.25);

        let mf = histogram.collect();
        let mut buffer = vec![];
        let encoder = CborEncoder::new();
        encoder.encode(&mf, &mut buffer)?;

        let mf: Vec<MetricFamily> = DagCborCodec.decode(&buffer)?;
        assert_eq!(
            mf,
            vec![MetricFamily {
                name: "test_histogram".into(),
                help: "test help".into(),
                metrics: vec![Metric::Histogram(Histogram {
                    labels: vec![LabelPair {
                        name: "a".into(),
                        value: "1".into()
                    },],
                    timestamp_ms: 0,
                    sample_count: 1,
                    sample_sum: 0.25,
                    buckets: vec![
                        Bucket {
                            cumulative_count: 0,
                            upper_bound: 0.005
                        },
                        Bucket {
                            cumulative_count: 0,
                            upper_bound: 0.01
                        },
                        Bucket {
                            cumulative_count: 0,
                            upper_bound: 0.025
                        },
                        Bucket {
                            cumulative_count: 0,
                            upper_bound: 0.05
                        },
                        Bucket {
                            cumulative_count: 0,
                            upper_bound: 0.1
                        },
                        Bucket {
                            cumulative_count: 1,
                            upper_bound: 0.25
                        },
                        Bucket {
                            cumulative_count: 1,
                            upper_bound: 0.5
                        },
                        Bucket {
                            cumulative_count: 1,
                            upper_bound: 1.0
                        },
                        Bucket {
                            cumulative_count: 1,
                            upper_bound: 2.5
                        },
                        Bucket {
                            cumulative_count: 1,
                            upper_bound: 5.0
                        },
                        Bucket {
                            cumulative_count: 1,
                            upper_bound: 10.0
                        }
                    ],
                })]
            }]
        );
        Ok(())
    }

    #[test]
    fn test_cbor_encoder_summary() -> Result<()> {
        let mut metric_family = prometheus::proto::MetricFamily::default();
        metric_family.set_name("test_summary".to_string());
        metric_family.set_help("test help".to_string());
        metric_family.set_field_type(prometheus::proto::MetricType::SUMMARY);

        let mut summary = prometheus::proto::Summary::default();
        summary.set_sample_count(5.0 as u64);
        summary.set_sample_sum(15.0);

        let mut quantile1 = prometheus::proto::Quantile::default();
        quantile1.set_quantile(50.0);
        quantile1.set_value(3.0);

        let mut quantile2 = prometheus::proto::Quantile::default();
        quantile2.set_quantile(100.0);
        quantile2.set_value(5.0);

        summary.set_quantile(vec![quantile1, quantile2].into());

        let mut metric = prometheus::proto::Metric::default();
        metric.set_summary(summary);
        metric_family.set_metric(vec![metric].into());

        let mut buffer = vec![];
        let encoder = CborEncoder::new();
        encoder.encode(&[metric_family], &mut buffer)?;

        let mf: Vec<MetricFamily> = DagCborCodec.decode(&buffer)?;
        assert_eq!(
            mf,
            vec![MetricFamily {
                name: "test_summary".into(),
                help: "test help".into(),
                metrics: vec![Metric::Summary(Summary {
                    labels: vec![],
                    timestamp_ms: 0,
                    sample_count: 5,
                    sample_sum: 15.0,
                    quantiles: vec![
                        Quantile {
                            quantile: 50.0,
                            value: 3.0,
                        },
                        Quantile {
                            quantile: 100.0,
                            value: 5.0,
                        }
                    ],
                })],
            }],
        );
        Ok(())
    }
}
