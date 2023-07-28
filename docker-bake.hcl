group "default" {
  targets = ["restarter", "seitai"]
}

variable "VERSION" {
  default = "latest"
}

target "restarter" {
  target = "restarter"
  tags = [
    "ghcr.io/hexium310/restarter:latest",
    "ghcr.io/hexium310/restarter:${VERSION}",
  ]
}

target "seitai" {
  target = "seitai"
  tags = [
    "ghcr.io/hexium310/seitai:latest",
    "ghcr.io/hexium310/seitai:${VERSION}"
  ]
}
