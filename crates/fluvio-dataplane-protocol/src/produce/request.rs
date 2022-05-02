use std::fmt::Debug;
use std::marker::PhantomData;

use crate::batch::RawRecords;
use crate::core::Encoder;
use crate::core::Decoder;
use crate::derive::FluvioDefault;

use crate::api::Request;
use crate::record::RecordSet;
use crate::Isolation;

use super::ProduceResponse;

pub type DefaultProduceRequest = ProduceRequest<RecordSet<RawRecords>>;
pub type DefaultPartitionRequest = PartitionProduceData<RecordSet<RawRecords>>;
pub type DefaultTopicRequest = TopicProduceData<RecordSet<RawRecords>>;

#[derive(Encoder, Decoder, FluvioDefault, Debug)]
pub struct ProduceRequest<R>
where
    R: Encoder + Decoder + Default + Debug,
{
    /// The transactional ID, or null if the producer is not transactional.
    #[fluvio(min_version = 3)]
    pub transactional_id: Option<String>,

    /// The number of acknowledgments the producer requires the leader to have received before
    /// considering a request complete. Allowed values: 0 for no acknowledgments, 1 for only the
    /// leader and -1 for the full ISR.
    pub acks: i16,

    /// The timeout to await a response in milliseconds.
    pub timeout_ms: i32,

    /// Each topic to produce to.
    pub topics: Vec<TopicProduceData<R>>,
    pub data: PhantomData<R>,
}

impl<R> ProduceRequest<R>
where
    R: Encoder + Decoder + Default + Debug,
{
    /// Get isolation from `acks`. Possible `acks` values: -1, 0, 1.
    /// -1 is mapped to `ReadCommitted`.
    /// 0, 1 are mapped to `ReadUncommitted`.
    pub fn isolation(&self) -> Isolation {
        match self.acks {
            acks if acks < 0 => Isolation::ReadCommitted,
            _ => Isolation::ReadUncommitted,
        }
    }
}

impl<R> Request for ProduceRequest<R>
where
    R: Debug + Decoder + Encoder,
{
    const API_KEY: u16 = 0;

    const MIN_API_VERSION: i16 = 0;
    const MAX_API_VERSION: i16 = 7;
    const DEFAULT_API_VERSION: i16 = 7;

    type Response = ProduceResponse;
}

#[derive(Encoder, Decoder, FluvioDefault, Debug)]
pub struct TopicProduceData<R>
where
    R: Encoder + Decoder + Default + Debug,
{
    /// The topic name.
    pub name: String,

    /// Each partition to produce to.
    pub partitions: Vec<PartitionProduceData<R>>,
    pub data: PhantomData<R>,
}

#[derive(Encoder, Decoder, FluvioDefault, Debug)]
pub struct PartitionProduceData<R>
where
    R: Encoder + Decoder + Default + Debug,
{
    /// The partition index.
    pub partition_index: i32,

    /// The record data to be produced.
    pub records: R,
}

#[cfg(feature = "file")]
pub use file::*;

#[cfg(feature = "file")]
mod file {
    use std::io::Error as IoError;

    use tracing::trace;
    use bytes::BytesMut;

    use crate::core::Version;
    use crate::record::FileRecordSet;
    use crate::store::FileWrite;
    use crate::store::StoreValue;

    use super::*;

    pub type FileProduceRequest = ProduceRequest<FileRecordSet>;
    pub type FileTopicRequest = TopicProduceData<FileRecordSet>;
    pub type FilePartitionRequest = PartitionProduceData<FileRecordSet>;

    impl FileWrite for FileProduceRequest {
        fn file_encode(
            &self,
            src: &mut BytesMut,
            data: &mut Vec<StoreValue>,
            version: Version,
        ) -> Result<(), IoError> {
            trace!("file encoding produce request");
            self.transactional_id.encode(src, version)?;
            self.acks.encode(src, version)?;
            self.timeout_ms.encode(src, version)?;
            self.topics.file_encode(src, data, version)?;
            Ok(())
        }
    }

    impl FileWrite for FileTopicRequest {
        fn file_encode(
            &self,
            src: &mut BytesMut,
            data: &mut Vec<StoreValue>,
            version: Version,
        ) -> Result<(), IoError> {
            trace!("file encoding produce topic request");
            self.name.encode(src, version)?;
            self.partitions.file_encode(src, data, version)?;
            Ok(())
        }
    }

    impl FileWrite for FilePartitionRequest {
        fn file_encode(
            &self,
            src: &mut BytesMut,
            data: &mut Vec<StoreValue>,
            version: Version,
        ) -> Result<(), IoError> {
            trace!("file encoding for partition request");
            self.partition_index.encode(src, version)?;
            self.records.file_encode(src, data, version)?;
            Ok(())
        }
    }
}
