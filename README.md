# Cockpit Auth OIDC

Cockpit supports for [custom authentication](https://github.com/cockpit-project/cockpit/blob/main/doc/authentication.md).
This repository contains an implementation of a remote machine login binary that authenticates users using OIDC with the Authorization Code Flow.
The binary will always use SSH to connect to the machine, even if the machine is localhost.

Although it should work with any generic OIDC provider, it is only tested using self-hosted [Keycloak](https://www.keycloak.org).

Note: while this project is not inherently insecure, you should review the source code and authentication mechanism before deploying this to a production environment, to make sure that it meets all relevant security requirements.

## Cockpit configuration

Cockpit has some undocumented configuration options related to OAuth2.
When configured, the Cockpit login page will redirect to the configured URL, appending the login page as a redirect URL.
An example configuration is shown below.

`/etc/cockpit/cockpit.conf`
```
[OAuth]
URL = <OIDC_HOST>/auth?response_type=code&client_id=<CLIENT_ID>&scope=openid&nonce=unused
TokenParam = code

[Bearer]
Action = remote-login-ssh

[SSH-Login]
Command = /etc/cockpit/cockpit-auth-oidc
```

Optionally, you can include `ConnectToUnknownHosts = true` under the `SSH-Login` section to allow connecting to unknown hosts.
Otherwise, you need to add every host to `/etc/ssh/ssh_known_hosts` 

## SSH keys

The authentication happens with SSH keys. A directory is configured that contains an SSH private key for each user that uses the application.
The file name of each private key should be the user `preferred_username`. These private keys unfortunately have to be without a password, since there is no easy secure method of unlocking them.

Such a key can be generated using the `ssh-keygen` command. Make sure no password is provided.
```
ssh-keygen -t ed25519 -C '<username>@cockpit' -f '<username>'
```

These keys should be owned by the user that runs Cockpit (`cockpit-wsinstance` on Fedora and other RHEL-based systems).

## Environment variables

In addition to the Cockpit configuration, some environment variables are needed for the custom authentication binary.

| Key                        | Value                                                                                        |
|----------------------------|----------------------------------------------------------------------------------------------|
| COCKPIT_OIDC_CLIENT_ID     | The OIDC client ID.                                                                          |
| COCKPIT_OIDC_CLIENT_SECRET | The OIDC client secret.                                                                      |
| COCKPIT_OIDC_ISSUER_URL    | The OIDC issuer URL. Must be accessible from the user and from Cockpit backend.              |
| COCKPIT_OIDC_LOGIN_URL     | The Cockpit login base URL. This is required for building the redirect URL.                  |
| COCKPIT_OIDC_SSH_KEYS_PATH | The directory containing the user SSH private keys.                                          |
| SSH_AUTH_SOCK              | *Optional* SSH auth socket location. See [Fedora/RHEL](#fedora-and-rhel-based-distributions) |

In case there are authentication failures, enabling `G_MESSAGES_DEBUG=all` will give some detailed logs from Cockpit and this binary.

For Cockpit on Fedora or other RHEL-based distributions, these configuration options can be set in a Systemd override.

Create the file (and directory if it does not exist) `/usr/lib/systemd/system/cockpit-wsinstance-http.service.d/cockpit-oidc.conf`.
```
[Service]
Environment=COCKPIT_OIDC_CLIENT_ID=
Environment=COCKPIT_OIDC_CLIENT_SECRET=
Environment=COCKPIT_OIDC_ISSUER_URL=
Environment=COCKPIT_OIDC_LOGIN_URL=
Environment=COCKPIT_OIDC_SSH_KEYS_PATH=
Environment=SSH_AUTH_SOCK=
```

## Deployment

### Fedora and RHEL-based distributions

When Cockpit is deployed on bare metal Fedora or other RHEL-based distributions, there are some issues with using SSH in Cockpit by default.
This is caused by the fact that there is no `ssh-agent` running for the user that is running the Cockpit process (`cockpit-wsinstance`).
`ssh-agent` is pre-installed software that keeps a set of SSH private keys that can be added using `ssh-add` for a set amount of time.
This binary calls `ssh-add` with the user private key and `ssh-agent` keeps the private key for 30 seconds while Cockpit establishes the connection.

Running `ssh-agent` for the Cockpit user can easily be done with a Systemd service.

Create a Systemd service in a location such as `/usr/lib/systemd/system/cockpit-ssh-agent.service`. 
```
[Unit]
Description=Cockpit SSH agent

[Service]
Environment=SSH_AUTH_SOCK=/etc/cockpit/ssh-auth.socket
ExecStart=/usr/bin/ssh-agent -D -a $SSH_AUTH_SOCK
User=cockpit-wsinstance
Group=cockpit-wsinstance
```

The SSH_AUTH_SOCK environment variable (`/etc/cockpit/ssh-auth.socket` above) should also be provided to Cockpit.

### Other Linux distributions

For other Linux distributions, it will either work out of the box or require a similar solution to Fedora/RHEL. This is not tested.

### Docker

When Cockpit is deployed in Docker, the deployment is as straightforward as mounting the `cockpit.conf`, `cockpit-auth-oidc` binary and the SSH keys directory in the container.

## Remote hosts

By default, Cockpit will attempt to connect to `127.0.0.1`. In case a different host should be used, Cockpit supports adding it to the login URL after an `=`.

For example:
```
https://cockpit.some.host/=192.168.1.1
```

Or specify a custom user (this only affects the user that it connects to, not the SSH key that is chosen):
```
https://cockpit.some.host/=john@192.168.1.1
```

This host is also used for building the correct redirect URL in this binary. A problem arises here though when the host is specified as the default, `127.0.0.1`.
This gives the binary the same host as when you provide no host at all, so there is no way of knowing what the redirect URL should be.
Since providing no host at all is the most common scenario, the binary will assume no host was provided when it receives the `127.0.0.1` host.
In case you do manually provide this host in the URL, the authentication will fail.