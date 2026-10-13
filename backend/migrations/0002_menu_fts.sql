-- 0002_menu_fts.sql
-- Menu Full-Text Search (FTS5) and Allergen Verification Schema Evolution

-- 1. Add allergens_verified column to menu_items (default 1 = verified)
ALTER TABLE menu_items ADD COLUMN allergens_verified INTEGER NOT NULL DEFAULT 1 CHECK (allergens_verified IN (0, 1));

-- 2. Create FTS5 virtual table for full-text menu searching
CREATE VIRTUAL TABLE IF NOT EXISTS menu_items_fts USING fts5(
    item_id UNINDEXED,
    name,
    description,
    category_name,
    dietary_tags_text,
    tokenize = 'porter unicode61'
);

-- 3. Triggers for real-time FTS synchronization on menu_items changes
CREATE TRIGGER IF NOT EXISTS trg_menu_items_after_insert AFTER INSERT ON menu_items
BEGIN
    INSERT INTO menu_items_fts(item_id, name, description, category_name, dietary_tags_text)
    VALUES (
        new.id,
        new.name,
        new.description,
        COALESCE((SELECT name FROM menu_categories WHERE id = new.category_id), ''),
        COALESCE((
            SELECT GROUP_CONCAT(dt.name, ' ')
            FROM menu_item_dietary_tags midt
            JOIN dietary_tags dt ON dt.id = midt.tag_id
            WHERE midt.menu_item_id = new.id
        ), '')
    );
END;

CREATE TRIGGER IF NOT EXISTS trg_menu_items_after_delete AFTER DELETE ON menu_items
BEGIN
    DELETE FROM menu_items_fts WHERE item_id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_menu_items_after_update AFTER UPDATE ON menu_items
BEGIN
    DELETE FROM menu_items_fts WHERE item_id = old.id;
    INSERT INTO menu_items_fts(item_id, name, description, category_name, dietary_tags_text)
    VALUES (
        new.id,
        new.name,
        new.description,
        COALESCE((SELECT name FROM menu_categories WHERE id = new.category_id), ''),
        COALESCE((
            SELECT GROUP_CONCAT(dt.name, ' ')
            FROM menu_item_dietary_tags midt
            JOIN dietary_tags dt ON dt.id = midt.tag_id
            WHERE midt.menu_item_id = new.id
        ), '')
    );
END;

-- 4. Trigger for real-time FTS synchronization when dietary tags are associated
CREATE TRIGGER IF NOT EXISTS trg_menu_dietary_after_insert AFTER INSERT ON menu_item_dietary_tags
BEGIN
    DELETE FROM menu_items_fts WHERE item_id = new.menu_item_id;
    INSERT INTO menu_items_fts(item_id, name, description, category_name, dietary_tags_text)
    SELECT
        m.id,
        m.name,
        m.description,
        COALESCE((SELECT name FROM menu_categories WHERE id = m.category_id), ''),
        COALESCE((
            SELECT GROUP_CONCAT(dt.name, ' ')
            FROM menu_item_dietary_tags midt
            JOIN dietary_tags dt ON dt.id = midt.tag_id
            WHERE midt.menu_item_id = m.id
        ), '')
    FROM menu_items m
    WHERE m.id = new.menu_item_id;
END;
