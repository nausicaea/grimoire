-- Add up migration script here
CREATE TABLE "dns-recon" (id SERIAL, fqdn varchar(256) REFERENCES "cert-recon"("cert-name"), ips inet[]);
