#!/bin/bash

# Comunicado Production Deployment Script
# Usage: ./deploy.sh [environment] [options]

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_NAME="comunicado"
CONTAINER_REGISTRY="ghcr.io/olafkfreund"
IMAGE_NAME="${CONTAINER_REGISTRY}/${PROJECT_NAME}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage information
show_usage() {
    cat << EOF
Comunicado Deployment Script

Usage: $0 [ENVIRONMENT] [OPTIONS]

Environments:
    local       Deploy locally using Docker Compose
    staging     Deploy to staging environment
    production  Deploy to production environment
    k8s         Deploy to Kubernetes cluster

Options:
    -v, --version VERSION    Specify version to deploy (default: latest)
    -t, --tag TAG           Specify container tag (default: latest)
    -b, --build             Build container before deployment
    -p, --push              Push container to registry
    -c, --clean             Clean up before deployment
    -h, --help              Show this help message
    --dry-run               Show what would be done without executing
    --skip-tests            Skip running tests before deployment
    --force                 Force deployment even if health checks fail

Examples:
    $0 local --build
    $0 production --version v1.0.0 --push
    $0 k8s --tag latest --clean
    $0 staging --dry-run

Environment Variables:
    DOCKER_REGISTRY         Container registry URL
    KUBERNETES_NAMESPACE    Kubernetes namespace (default: comunicado)
    CONFIG_FILE             Custom configuration file path
EOF
}

# Parse command line arguments
parse_arguments() {
    ENVIRONMENT=""
    VERSION="latest"
    TAG="latest"
    BUILD=false
    PUSH=false
    CLEAN=false
    DRY_RUN=false
    SKIP_TESTS=false
    FORCE=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            local|staging|production|k8s)
                ENVIRONMENT="$1"
                shift
                ;;
            -v|--version)
                VERSION="$2"
                TAG="$2"
                shift 2
                ;;
            -t|--tag)
                TAG="$2"
                shift 2
                ;;
            -b|--build)
                BUILD=true
                shift
                ;;
            -p|--push)
                PUSH=true
                shift
                ;;
            -c|--clean)
                CLEAN=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    if [[ -z "$ENVIRONMENT" ]]; then
        log_error "Environment must be specified"
        show_usage
        exit 1
    fi
}

# Execute command with dry run support
execute() {
    if [[ "$DRY_RUN" == true ]]; then
        log_info "[DRY RUN] Would execute: $*"
    else
        log_info "Executing: $*"
        "$@"
    fi
}

# Check prerequisites
check_prerequisites() {
    local required_tools=()
    
    case "$ENVIRONMENT" in
        local)
            required_tools=("docker" "docker-compose")
            ;;
        staging|production)
            required_tools=("docker" "ssh" "rsync")
            ;;
        k8s)
            required_tools=("kubectl" "helm")
            ;;
    esac

    # Always check for common tools
    required_tools+=("git" "cargo")

    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "$tool is required but not installed"
            exit 1
        fi
    done

    # Check Docker daemon
    if [[ " ${required_tools[*]} " =~ " docker " ]]; then
        if ! docker info &> /dev/null; then
            log_error "Docker daemon is not running"
            exit 1
        fi
    fi

    # Check Kubernetes connection
    if [[ "$ENVIRONMENT" == "k8s" ]]; then
        if ! kubectl cluster-info &> /dev/null; then
            log_error "Cannot connect to Kubernetes cluster"
            exit 1
        fi
    fi

    log_success "Prerequisites check passed"
}

# Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == true ]]; then
        log_warning "Skipping tests as requested"
        return
    fi

    log_info "Running tests..."
    
    execute cargo test --all-features -- \
        --skip test_imap_connection \
        --skip test_oauth_flow \
        --skip test_caldav_sync \
        --skip test_network_image_loading

    log_success "Tests passed"
}

# Build container image
build_container() {
    if [[ "$BUILD" != true ]]; then
        return
    fi

    log_info "Building container image..."
    
    execute docker build \
        -t "${IMAGE_NAME}:${TAG}" \
        -t "${IMAGE_NAME}:latest" \
        .

    # Run container security scan
    if command -v trivy &> /dev/null; then
        log_info "Running security scan..."
        execute trivy image "${IMAGE_NAME}:${TAG}"
    fi

    log_success "Container image built: ${IMAGE_NAME}:${TAG}"
}

# Push container image
push_container() {
    if [[ "$PUSH" != true ]]; then
        return
    fi

    log_info "Pushing container image..."
    
    execute docker push "${IMAGE_NAME}:${TAG}"
    if [[ "$TAG" != "latest" ]]; then
        execute docker push "${IMAGE_NAME}:latest"
    fi

    log_success "Container image pushed"
}

# Clean up resources
cleanup() {
    if [[ "$CLEAN" != true ]]; then
        return
    fi

    log_info "Cleaning up..."
    
    case "$ENVIRONMENT" in
        local)
            execute docker-compose down --volumes --remove-orphans
            execute docker system prune -f
            ;;
        k8s)
            execute kubectl delete --ignore-not-found=true -f k8s/
            ;;
    esac

    log_success "Cleanup completed"
}

# Deploy to local environment
deploy_local() {
    log_info "Deploying to local environment..."
    
    cleanup
    
    # Use development profile if available
    if [[ -f "docker-compose.override.yml" ]]; then
        execute docker-compose up -d
    else
        execute docker-compose -f docker-compose.yml up -d
    fi

    # Wait for services to be ready
    log_info "Waiting for services to be ready..."
    sleep 10

    # Health check
    if execute docker-compose ps | grep -q "Up (healthy)"; then
        log_success "Local deployment completed successfully"
        log_info "Access application: docker-compose exec comunicado bash"
    else
        log_error "Health check failed"
        execute docker-compose logs comunicado
        exit 1
    fi
}

# Deploy to Kubernetes
deploy_k8s() {
    local namespace="${KUBERNETES_NAMESPACE:-comunicado}"
    
    log_info "Deploying to Kubernetes cluster..."
    log_info "Namespace: $namespace"
    
    # Create namespace if it doesn't exist
    execute kubectl create namespace "$namespace" --dry-run=client -o yaml | kubectl apply -f -

    # Update image tag in manifests
    if [[ -d "k8s" ]]; then
        find k8s -name "*.yaml" -exec sed -i.bak "s|${IMAGE_NAME}:.*|${IMAGE_NAME}:${TAG}|g" {} \;
        
        # Apply manifests
        execute kubectl apply -f k8s/ -n "$namespace"
        
        # Restore original manifests
        find k8s -name "*.yaml.bak" -exec sh -c 'mv "$1" "${1%.bak}"' _ {} \;
    else
        log_error "Kubernetes manifests not found in k8s/ directory"
        exit 1
    fi

    # Wait for deployment to be ready
    log_info "Waiting for deployment to be ready..."
    execute kubectl rollout status deployment/comunicado -n "$namespace" --timeout=300s

    # Health check
    if kubectl get pods -n "$namespace" -l app=comunicado | grep -q "Running"; then
        log_success "Kubernetes deployment completed successfully"
        log_info "Access pod: kubectl exec -it -n $namespace deployment/comunicado -- bash"
    else
        log_error "Deployment failed"
        execute kubectl get events -n "$namespace" --sort-by=.metadata.creationTimestamp
        exit 1
    fi
}

# Deploy to staging/production
deploy_remote() {
    local env_config="${SCRIPT_DIR}/deploy-${ENVIRONMENT}.env"
    
    if [[ -f "$env_config" ]]; then
        source "$env_config"
    else
        log_error "Environment configuration not found: $env_config"
        exit 1
    fi

    log_info "Deploying to $ENVIRONMENT environment..."
    log_info "Target: ${DEPLOY_HOST:-unknown}"

    # Copy deployment files to remote host
    if [[ -n "${DEPLOY_HOST:-}" ]]; then
        execute rsync -avz \
            --exclude='.git' \
            --exclude='target' \
            --exclude='node_modules' \
            . "${DEPLOY_USER}@${DEPLOY_HOST}:${DEPLOY_PATH}"

        # Execute remote deployment
        execute ssh "${DEPLOY_USER}@${DEPLOY_HOST}" \
            "cd ${DEPLOY_PATH} && ./deploy.sh local --tag ${TAG}"
    else
        log_error "DEPLOY_HOST not configured"
        exit 1
    fi

    log_success "Remote deployment completed"
}

# Health check
health_check() {
    log_info "Running health checks..."
    
    case "$ENVIRONMENT" in
        local)
            if docker-compose exec -T comunicado comunicado --health-check; then
                log_success "Health check passed"
            else
                log_error "Health check failed"
                return 1
            fi
            ;;
        k8s)
            local namespace="${KUBERNETES_NAMESPACE:-comunicado}"
            if kubectl exec -n "$namespace" deployment/comunicado -- comunicado --health-check; then
                log_success "Health check passed"
            else
                log_error "Health check failed"
                return 1
            fi
            ;;
    esac
}

# Main deployment function
main() {
    parse_arguments "$@"
    
    log_info "Starting deployment..."
    log_info "Environment: $ENVIRONMENT"
    log_info "Version: $VERSION"
    log_info "Tag: $TAG"
    log_info "Build: $BUILD"
    log_info "Push: $PUSH"
    log_info "Clean: $CLEAN"
    log_info "Dry run: $DRY_RUN"
    
    check_prerequisites
    
    if [[ "$DRY_RUN" != true ]]; then
        run_tests
    fi
    
    build_container
    push_container
    
    case "$ENVIRONMENT" in
        local)
            deploy_local
            ;;
        k8s)
            deploy_k8s
            ;;
        staging|production)
            deploy_remote
            ;;
        *)
            log_error "Unknown environment: $ENVIRONMENT"
            exit 1
            ;;
    esac
    
    if [[ "$FORCE" != true ]]; then
        if ! health_check; then
            log_error "Deployment failed health check"
            exit 1
        fi
    fi
    
    log_success "Deployment completed successfully!"
    log_info "Environment: $ENVIRONMENT"
    log_info "Version: $VERSION"
    log_info "Image: ${IMAGE_NAME}:${TAG}"
}

# Run main function with all arguments
main "$@"