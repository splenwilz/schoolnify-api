# Health Check

## `GET /health`

Check API and database health.

**Response `200`:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "checks": {
    "database": "up"
  }
}
```

**Response `503`:** Same structure with `"unhealthy"` / `"down"`.
