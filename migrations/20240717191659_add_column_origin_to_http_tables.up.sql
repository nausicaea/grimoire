-- Add up migration script here
ALTER TABLE "http-recon" ADD COLUMN domain VARCHAR(256);
ALTER TABLE "https-recon" ADD COLUMN domain VARCHAR(256);
UPDATE "http-recon" SET domain = (SELECT regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1') AS domain FROM "http-recon") WHERE domain IS NULL;
UPDATE "https-recon" SET domain = (SELECT regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1') AS domain FROM "https-recon") WHERE domain IS NULL;
