-- Add down migration script here
ALTER TABLE "http-recon" DROP COLUMN domain;
UPDATE "http-recon" SET domain = (SELECT regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1') AS domain FROM "http-recon") WHERE domain IS NULL;

ALTER TABLE "https-recon" DROP COLUMN domain;
UPDATE "https-recon" SET domain = (SELECT regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1') AS domain FROM "https-recon") WHERE domain IS NULL;
