CREATE TABLE image_processor_job (
    id UUID PRIMARY KEY,
    hold_until TIMESTAMP WITH TIME ZONE,
    priority INTEGER NOT NULL,
    claimed_by_id UUID,
    task bytea NOT NULL
);

CREATE INDEX image_processor_job_hold_until_index ON image_processor_job (hold_until ASC);
CREATE INDEX image_processor_job_priority_index ON image_processor_job (priority DESC, id DESC);
