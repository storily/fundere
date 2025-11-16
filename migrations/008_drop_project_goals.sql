-- Drop the CHECK constraint on goal
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_goal_check;

-- Drop the goal column
ALTER TABLE projects DROP COLUMN goal;
