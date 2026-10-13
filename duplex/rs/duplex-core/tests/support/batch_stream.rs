use duplex_core::{BatchStream, Error, OutboundBatch};

pub fn streamed(batches: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();

    for batch in batches {
        let mut batch = OutboundBatch::from(batch.to_vec());

        BatchStream::frame(&mut batch);
        bytes.extend_from_slice(batch.as_slice());
    }

    bytes
}

pub fn batches_of(stream: &mut BatchStream, bytes: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    let mut batches = Vec::new();

    stream.process(bytes, |batch| batches.push(batch.as_slice().to_vec()))?;

    Ok(batches)
}
