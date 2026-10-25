-- The commerce schema.
--
-- Applied by `commerce_core::db::migrate` on every service that has a
-- database, and idempotent: every statement is `IF NOT EXISTS`, so a
-- service can apply it on boot and a second service can apply it again.
--
-- ## Why the shapes are what they are
--
-- **Ids are text, not uuid.** The design doc's schema proposal
-- (`docs/ecommerce/03-detailed/API・DB・状態機械設計.md` §2) used uuid, but
-- the services mint readable ids — `sku-bra-01`, `ord-000001`,
-- `sess-000001` — and those ids are already in the payloads, the URLs and
-- the tests. A schema that disagreed with them would have needed a
-- translation layer in every service, and the ids would stop being
-- readable in the places a person reads them.
--
-- **Times are bigint milliseconds, not timestamptz.** Every service
-- already keeps its clock as `i64` milliseconds since the epoch
-- (`commerce_core::idempotency::now_ms`), and the contract sends
-- `expires_at_ms` and `authorized_at_ms` in that form. A timestamp column
-- would mean converting at every boundary and in every comparison.
--
-- The constraints are the ones the mock proves
-- (`services/crates/commerce-mocks/src/postgres.rs`, CASE-IT-001): stock
-- cannot go negative, a reservation cannot exceed the stock on hand, and
-- an order cannot reserve the same SKU twice.

-- ---------------------------------------------------------------------------
-- identity service
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS identity_session (
    session_id    text PRIMARY KEY,
    kind          text        NOT NULL CHECK (kind IN ('guest', 'line_user')),
    channel       text        NOT NULL CHECK (channel IN ('web', 'app', 'line')),
    subject       text,
    issued_at_ms  bigint      NOT NULL,
    expires_at_ms bigint      NOT NULL,
    revoked       boolean     NOT NULL DEFAULT false
);

-- The id counter, so two instances cannot mint the same session id.
CREATE SEQUENCE IF NOT EXISTS identity_session_seq;

-- Nonces already consumed: a captured id_token must not be replayed.
CREATE TABLE IF NOT EXISTS consumed_nonce (
    nonce      text PRIMARY KEY,
    used_at_ms bigint NOT NULL
);

-- ---------------------------------------------------------------------------
-- cart service
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS cart (
    cart_id    text PRIMARY KEY,
    owner_id   text,
    version    integer     NOT NULL DEFAULT 1 CHECK (version >= 1),
    expires_at_ms bigint  NOT NULL
);

CREATE TABLE IF NOT EXISTS cart_item (
    cart_id              text   NOT NULL REFERENCES cart (cart_id) ON DELETE CASCADE,
    sku_id               text   NOT NULL,
    quantity             integer NOT NULL CHECK (quantity > 0),
    -- Display only. The checkout re-prices every line from the catalogue.
    unit_amount_snapshot bigint NOT NULL,
    currency             char(3) NOT NULL,
    PRIMARY KEY (cart_id, sku_id)
);

-- ---------------------------------------------------------------------------
-- inventory service
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS stock (
    sku_id   text PRIMARY KEY,
    on_hand  bigint NOT NULL CHECK (on_hand >= 0),
    reserved bigint NOT NULL CHECK (reserved >= 0 AND reserved <= on_hand)
);

CREATE TABLE IF NOT EXISTS inventory_reservation (
    reservation_id text PRIMARY KEY,
    sku_id         text   NOT NULL,
    order_id       text   NOT NULL,
    quantity       bigint NOT NULL CHECK (quantity > 0),
    state          text   NOT NULL CHECK (
        state IN ('pending', 'reserved', 'committed', 'released', 'expired')
    ),
    expires_at_ms  bigint NOT NULL,
    -- A second reservation of the same SKU for the same order is
    -- structurally impossible, not merely unlikely.
    UNIQUE (order_id, sku_id)
);

-- ---------------------------------------------------------------------------
-- order service
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS commerce_order (
    order_id          text PRIMARY KEY,
    buyer_id          text   NOT NULL,
    state             text   NOT NULL,
    total_amount_minor bigint NOT NULL,
    total_currency     char(3) NOT NULL,
    created_at_ms      bigint NOT NULL,
    updated_at_ms      bigint NOT NULL
);

-- The columns the order carries besides its money and its state. They were
-- added after the table was first created, so they arrive as `ADD COLUMN
-- IF NOT EXISTS` — idempotent, and a deployment that already has the table
-- picks them up on the next boot.
ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS reservation_id text;
ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS provider_ref text;
ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS transitions_used integer NOT NULL DEFAULT 0;
ALTER TABLE commerce_order ADD COLUMN IF NOT EXISTS awaiting_reconciliation boolean NOT NULL DEFAULT false;

-- The id counter for the orders, the checkouts, the reservations and the
-- provider references. One sequence for all of them, so the ids of one
-- checkout are neighbours and are easy to read together.
CREATE SEQUENCE IF NOT EXISTS order_seq;

CREATE TABLE IF NOT EXISTS order_line (
    order_id        text    NOT NULL REFERENCES commerce_order (order_id) ON DELETE CASCADE,
    sku_id          text    NOT NULL,
    quantity        integer NOT NULL CHECK (quantity > 0),
    unit_amount_minor bigint NOT NULL,
    currency        char(3) NOT NULL,
    PRIMARY KEY (order_id, sku_id)
);

-- The trail: one row per transition, in order.
CREATE TABLE IF NOT EXISTS order_event (
    order_id   text   NOT NULL REFERENCES commerce_order (order_id) ON DELETE CASCADE,
    sequence   integer NOT NULL,
    event      text   NOT NULL,
    from_state text   NOT NULL,
    to_state   text   NOT NULL,
    at_ms      bigint NOT NULL,
    PRIMARY KEY (order_id, sequence)
);

CREATE TABLE IF NOT EXISTS checkout (
    checkout_id   text PRIMARY KEY,
    order_id      text NOT NULL,
    state         text NOT NULL,
    created_at_ms bigint NOT NULL,
    updated_at_ms bigint NOT NULL
);

-- The idempotency record: one key, one payload, one stored answer.
CREATE TABLE IF NOT EXISTS idempotency_record (
    actor     text   NOT NULL,
    operation text   NOT NULL,
    key       text   NOT NULL,
    fingerprint text NOT NULL,
    response  text   NOT NULL,
    status    integer NOT NULL,
    at_ms     bigint NOT NULL,
    PRIMARY KEY (actor, operation, key)
);

-- When the record stops counting. The rule it implements is "a duplicate
-- control that expires", so the expiry is part of the record.
ALTER TABLE idempotency_record ADD COLUMN IF NOT EXISTS expires_at_ms bigint NOT NULL DEFAULT 0;

-- The outbox: events the order service produced, not yet published.
CREATE TABLE IF NOT EXISTS outbox_event (
    id            bigserial PRIMARY KEY,
    topic         text NOT NULL,
    payload       text NOT NULL,
    published_at_ms bigint,
    created_at_ms bigint NOT NULL
);

-- The order the event belongs to, so an order's unpublished events can be
-- found without reading the payload.
ALTER TABLE outbox_event ADD COLUMN IF NOT EXISTS order_id text;

CREATE INDEX IF NOT EXISTS outbox_event_unpublished
    ON outbox_event (created_at_ms) WHERE published_at_ms IS NULL;

-- ---------------------------------------------------------------------------
-- payment adapter
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS payment_attempt (
    attempt_id    text PRIMARY KEY,
    order_id      text NOT NULL,
    provider_ref  text,
    amount_minor  bigint NOT NULL,
    currency      char(3) NOT NULL,
    state         text NOT NULL,
    created_at_ms bigint NOT NULL
);

CREATE INDEX IF NOT EXISTS payment_attempt_by_order ON payment_attempt (order_id);
