export const TRANSPORT_HEADER = 'duplex-transport';
export const RECEIVED_HEADER = 'duplex-received';
export const BATCHES_CONTENT_TYPE = 'application/octet-stream';

export const EVENTS_PATH = '/events';
export const STREAM_PATH = '/stream';

export const NO_CONTENT = 204;

export const EVENT_END = '\n\n';
export const LINE_END = '\n';
export const FIELD_SEPARATOR = ': ';
export const ID_FIELD = 'id';
export const DATA_FIELD = 'data';

export const VARUINT_PAYLOAD_MASK = 0x7f;
export const VARUINT_CONTINUATION = 0x80;

export const NATIVE_BASE64_MIN_LENGTH = 256;
