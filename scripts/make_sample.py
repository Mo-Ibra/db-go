#!/usr/bin/env python3
"""Create a sample SQLite database for testing DBGo.

Usage:
    python3 scripts/make_sample.py
"""

from __future__ import annotations

import os
import random
import sqlite3
from datetime import datetime, timedelta, timezone

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "samples", "companies.db")

FIRST = [
    "Acme", "Globex", "Initech", "Umbrella", "Hooli", "Vehement", "Massive",
    "Vandelay", "Wonka", "Stark", "Wayne", "Cyberdyne", "Soylent", "Tyrell",
    "Aperture", "Black Mesa", "Gringotts", "Duff", "Monarch", "Nakatomi",
]
SECOND = [
    "Labs", "Systems", "Industries", "Dynamics", "Analytics", "Robotics",
    "Networks", "Holdings", "Group", "Works",
]
CITIES = [
    "Berlin", "Austin", "Tokyo", "London", "Toronto", "Paris", "Lisbon",
    "Singapore", "New York", "Cape Town", "São Paulo", "Sydney",
]
TAGS = ["b2b", "saas", "ai", "infra", "fintech", "devtools", "open-source", "hardware"]


def main() -> None:
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    if os.path.exists(OUT):
        os.remove(OUT)

    conn = sqlite3.connect(OUT)
    conn.executescript(
        """
        PRAGMA journal_mode=WAL;

        CREATE TABLE companies (
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            domain      TEXT UNIQUE,
            city        TEXT,
            employees   INTEGER,
            revenue     REAL,
            tags        TEXT,
            active      INTEGER NOT NULL DEFAULT 1,
            founded     TEXT,
            notes       TEXT,
            logo        BLOB,
            updated_at  TEXT NOT NULL
        );

        CREATE TABLE links (
            id          INTEGER PRIMARY KEY,
            company_id  INTEGER NOT NULL REFERENCES companies(id),
            url         TEXT NOT NULL,
            kind        TEXT NOT NULL,
            checked_at  TEXT
        );

        CREATE INDEX idx_companies_city ON companies(city);
        CREATE INDEX idx_links_company ON links(company_id);
        """
    )

    now = datetime.now(timezone.utc)
    random.seed(7)
    companies = []
    for i in range(1, 10_001):
        name = f"{random.choice(FIRST)} {random.choice(SECOND)}"
        domain = f"{name.lower().replace(' ', '')}-{i}.test"
        city = random.choice(CITIES)
        employees = random.choice([None, random.randint(1, 50), random.randint(50, 2_000), random.randint(2_000, 90_000)])
        revenue = None if random.random() < 0.2 else round(random.uniform(0.1, 900.0), 2)
        tags = ",".join(random.sample(TAGS, random.randint(1, 3)))
        active = random.choice([0, 1, 1, 1])
        founded = f"{random.randint(1985, 2025)}-{random.randint(1, 12):02d}-{random.randint(1, 28):02d}"
        notes = None if random.random() < 0.4 else f"Researched via document batch {random.randint(1, 40)}."
        logo = bytes([random.randint(0, 255) for _ in range(random.choice([0, 16, 64, 256]))])
        updated = (now - timedelta(minutes=random.randint(0, 60 * 24 * 30))).isoformat()
        companies.append((i, name, domain, city, employees, revenue, tags, active, founded, notes, logo, updated))

    conn.executemany(
        """
        INSERT INTO companies
            (id, name, domain, city, employees, revenue, tags, active, founded, notes, logo, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        companies,
    )

    links = []
    for i in range(1, 30_001):
        company_id = random.randint(1, 10_000)
        kind = random.choice(["website", "linkedin", "github", "crunchbase", "news"])
        links.append(
            (
                i,
                company_id,
                f"https://example.test/{kind}/{company_id}/{i}",
                kind,
                (now - timedelta(days=random.randint(0, 365))).isoformat(),
            )
        )

    conn.executemany(
        "INSERT INTO links (id, company_id, url, kind, checked_at) VALUES (?, ?, ?, ?, ?)",
        links,
    )
    conn.commit()
    conn.close()
    print(f"wrote {OUT} ({os.path.getsize(OUT):,} bytes)")


if __name__ == "__main__":
    main()
