resource "kubernetes_namespace" "sqlx" {
  metadata {
    name = var.namespace
  }
}

resource "kubernetes_config_map" "sqlx" {
  metadata {
    name      = "${var.name}-migrations"
    namespace = kubernetes_namespace.sqlx.metadata.0.name
  }

  data = {
    for file in fileset(var.migrations_path, "*.sql") : file => file("${var.migrations_path}/${file}")
  }
}

resource "kubernetes_job" "sqlx" {
  metadata {
    name      = var.name
    namespace = kubernetes_namespace.sqlx.metadata.0.name
  }

  wait_for_completion = true
  timeouts {
    create = "10m"
  }

  lifecycle {
    replace_triggered_by = [
      kubernetes_config_map.sqlx,
    ]
  }

  spec {
    completions = 1
    template {
      metadata {}
      spec {
        init_container {
          name  = "migrations"
          image = "busybox"
          volume_mount {
            name       = "migrations"
            mount_path = "/migrations"
          }
          volume_mount {
            name       = "migration-files"
            mount_path = "/migration-files"
          }
          command = [
            "sh",
            "-c",
            "cp /migration-files/* /migrations/."
          ]
        }

        container {
          name  = "sqlx-migrations"
          image = var.docker_image
          env {
            name  = "DATABASE_URL"
            value = "postgres://${var.postgres_username}@${var.postgres_host}/${var.postgres_database}?sslmode=verify-full&sslrootcert=/tls/ca.crt&sslcert=/tls/tls.crt&sslkey=/tls/tls.key"
          }
          volume_mount {
            name       = "tls"
            mount_path = "/tls"
          }
          volume_mount {
            name       = "migrations"
            mount_path = "/migrations"
          }
          args = [
            "-c",
            "sqlx database create && sqlx migrate info --source /migrations && sqlx migrate run --source /migrations",
          ]
        }
        volume {
          name = "tls"
          secret {
            default_mode = "0400"
            secret_name  = var.tls_secret_name
          }
        }
        volume {
          name = "migrations"
          empty_dir {}
        }
        volume {
          name = "migration-files"
          config_map {
            default_mode = "0400"
            name         = kubernetes_config_map.sqlx.metadata.0.name
          }
        }
      }
    }
  }
}
