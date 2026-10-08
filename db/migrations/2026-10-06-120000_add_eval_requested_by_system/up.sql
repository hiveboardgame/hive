-- Evals the site starts on its own when the queue is idle; users always go first.
alter table game_evals add column requested_by_system boolean not null default false;
