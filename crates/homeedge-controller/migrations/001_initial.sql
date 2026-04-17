PRAGMA foreign_keys = ON;

CREATE TABLE nodes (
    node_id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    last_heartbeat TEXT NULL,
    capabilities_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE services (
    service_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    selector TEXT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_services_name_version
ON services(name, version);

CREATE TABLE assignments (
    service_id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(service_id) REFERENCES services(service_id) ON DELETE CASCADE,
    FOREIGN KEY(node_id) REFERENCES nodes(node_id) ON DELETE CASCADE
);

CREATE TABLE observed_service_status (
    node_id TEXT NOT NULL,
    service_id TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (node_id, service_id),
    FOREIGN KEY(node_id) REFERENCES nodes(node_id) ON DELETE CASCADE,
    FOREIGN KEY(service_id) REFERENCES services(service_id) ON DELETE CASCADE
);

CREATE INDEX idx_nodes_status ON nodes(status);
CREATE INDEX idx_assignments_node_id ON assignments(node_id);
CREATE INDEX idx_observed_service_status_node_id
    ON observed_service_status(node_id);