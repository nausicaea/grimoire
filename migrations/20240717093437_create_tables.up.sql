-- Add up migration script here
CREATE TABLE "cert-recon" (id SERIAL, domain varchar(256) NOT NULL, "cert-name" varchar(256) PRIMARY KEY);
CREATE TABLE "dns-recon" (id SERIAL, domain varchar(256) NOT NULL, fqdn varchar(256) PRIMARY KEY, ips inet[]);
CREATE TABLE "http-recon" (id SERIAL, domain varchar(256) NOT NULL, fqdn varchar(256) PRIMARY KEY, url text NOT NULL, "response-status" smallint NOT NULL, headers jsonb NOT NULL DEFAULT '{}'::jsonb);
CREATE TABLE "https-recon" (id SERIAL, domain varchar(256) NOT NULL, fqdn varchar(256) PRIMARY KEY, url text NOT NULL, "response-status" smallint NOT NULL, headers jsonb NOT NULL DEFAULT '{}'::jsonb);
