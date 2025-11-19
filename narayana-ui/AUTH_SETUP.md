# Authentication Setup

The NarayanaDB admin dashboard now requires authentication. 

## Environment Variables

Set the following environment variables before starting the server:

```bash
export NARAYANA_ADMIN_USER=admin
export NARAYANA_ADMIN_PASSWORD=your-secure-password
```

Or create a `.env` file in the project root (if your server supports it).

## Default Credentials

If environment variables are not set, the default credentials are:
- Username: `admin`
- Password: `admin`

**⚠️ Warning**: Change these defaults in production!

## Usage

1. Start the server with environment variables:
   ```bash
   NARAYANA_ADMIN_USER=myuser NARAYANA_ADMIN_PASSWORD=mypass cargo run --bin narayana-server
   ```

2. Access the UI at http://localhost:3000
3. You will be redirected to the login page
4. Enter your credentials to access the dashboard

## Features

- ✅ Beautiful React login screen
- ✅ Protected routes (all dashboard pages require authentication)
- ✅ Persistent login (session stored in localStorage)
- ✅ Logout functionality
- ✅ Automatic redirect to login if not authenticated
- ✅ Automatic redirect to dashboard if already authenticated

