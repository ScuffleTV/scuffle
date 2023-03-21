output "hostname" {
  value = "${kubernetes_service.api.metadata[0].name}.${var.namespace}.svc.cluster.local"
}
