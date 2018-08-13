CREATE TABLE IF NOT EXISTS process_manager (
        entry_id SERIAL NOT NULL UNIQUE,
        pid INTEGER NOT NULL, -- pid of daemon
        ip VARCHAR NOT NULL, -- ip where daemon is running
        status VARCHAR(20), -- status of the daemon
        start_time BIGINT NOT NULL DEFAULT extract(epoch FROM now())::BIGINT,-- epoch when it started
        PRIMARY KEY (ip,pid)
        );


CREATE TABLE IF NOT EXISTS regions (
    region_id SERIAL NOT NULL UNIQUE,
    region_name VARCHAR (256) PRIMARY KEY NOT NULL 
    );

CREATE TABLE IF NOT EXISTS storage_types (
    storage_id SERIAL NOT NULL UNIQUE,
    storage_type VARCHAR (256) PRIMARY KEY NOT NULL
    );

INSERT INTO storage_types (storage_type) vALUES ('ceph');
INSERT INTO storage_types (storage_type) vALUES ('sio');
INSERT INTO storage_types (storage_type) vALUES ('solidfire');
INSERT INTO storage_types (storage_type) VALUES ('hitachi');

CREATE TABLE IF NOT EXISTS storage_details (
    detail_id SERIAL NOT NULL UNIQUE,
    storage_id INTEGER REFERENCES storage_types(storage_id) ON DELETE CASCADE,
    region_id INTEGER REFERENCES regions(region_id) ON DELETE CASCADE,
    hostname VARCHAR (512),
    name_key1 VARCHAR, -- name of storage array
    uuid VARCHAR,
    name_key2 VARCHAR,-- name of pool, switch etc
    PRIMARY KEY (region_id, storage_id, hostname, name_key1, name_key2)
    );

CREATE TABLE IF NOT EXISTS disks (
    disk_uuid VARCHAR NOT NULL,
    detail_id INTEGER REFERENCES storage_details(detail_id) ON DELETE CASCADE,
    disk_name VARCHAR,
    disk_path VARCHAR
    );
