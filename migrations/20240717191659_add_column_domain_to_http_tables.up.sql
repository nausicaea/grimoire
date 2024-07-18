-- Add up migration script here
ALTER TABLE "http-recon" ADD COLUMN domain VARCHAR(256);
ALTER TABLE "https-recon" ADD COLUMN domain VARCHAR(256);
UPDATE "http-recon" SET domain = regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1');
UPDATE "https-recon" SET domain = regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1');
