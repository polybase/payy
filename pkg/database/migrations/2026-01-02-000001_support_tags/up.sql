CREATE TABLE support_tags (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "name" TEXT UNIQUE NOT NULL,
    "display_name" TEXT NOT NULL,
    "color" TEXT NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE support_issue_tags (
    "support_issue_id" UUID NOT NULL,
    "support_tag_id" UUID NOT NULL,
    "added_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("support_issue_id", "support_tag_id"),
    CONSTRAINT fk_support_issue FOREIGN KEY ("support_issue_id")
        REFERENCES support_issues("id") ON DELETE CASCADE,
    CONSTRAINT fk_support_tag FOREIGN KEY ("support_tag_id")
        REFERENCES support_tags("id") ON DELETE CASCADE
);

CREATE INDEX idx_support_tags_is_active ON support_tags (is_active);
CREATE INDEX idx_support_issue_tags_tag_id ON support_issue_tags (support_tag_id);
