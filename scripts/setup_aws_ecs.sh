# 1. First, set your AWS region
export AWS_REGION=eu-west-2  # or your preferred region

# 2. Create an ECR repository
aws ecr create-repository \
    --repository-name anypay-websockets \
    --region $AWS_REGION

# 3. Create an ECS cluster
aws ecs create-cluster \
    --cluster-name anypay-websockets-cluster

# 4. Create a task execution role
aws iam create-role \
    --role-name ecsTaskExecutionRole \
    --assume-role-policy-document '{
      "Version": "2012-10-17",
      "Statement": [
        {
          "Effect": "Allow",
          "Principal": {
            "Service": "ecs-tasks.amazonaws.com"
          },
          "Action": "sts:AssumeRole"
        }
      ]
    }'

# 5. Attach the task execution policy
aws iam attach-role-policy \
    --role-name ecsTaskExecutionRole \
    --policy-arn arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy

# 6. Create a task definition JSON file (.aws/task-definition.json):
mkdir -p .aws
cat > .aws/task-definition.json << 'EOF'
{
    "family": "anypay-websockets",
    "networkMode": "awsvpc",
    "requiresCompatibilities": ["FARGATE"],
    "cpu": "256",
    "memory": "512",
    "executionRoleArn": "arn:aws:iam::<YOUR_ACCOUNT_ID>:role/ecsTaskExecutionRole",
    "containerDefinitions": [
        {
            "name": "anypay-websockets",
            "image": "<YOUR_ACCOUNT_ID>.dkr.ecr.<YOUR_REGION>.amazonaws.com/anypay-websockets:latest",
            "portMappings": [
                {
                    "containerPort": 8080,
                    "protocol": "tcp"
                }
            ],
            "essential": true,
            "logConfiguration": {
                "logDriver": "awslogs",
                "options": {
                    "awslogs-group": "/ecs/anypay-websockets",
                    "awslogs-region": "<YOUR_REGION>",
                    "awslogs-stream-prefix": "ecs"
                }
            }
        }
    ]
}
EOF

# 7. Create a log group for the container
aws logs create-log-group \
    --log-group-name /ecs/anypay-websockets

# 8. Register the task definition
aws ecs register-task-definition \
    --cli-input-json file://.aws/task-definition.json

# 9. Create a VPC (if you don't have one)
VPC_ID=$(aws ec2 create-vpc \
    --cidr-block 10.0.0.0/16 \
    --query 'Vpc.VpcId' \
    --output text)

# 10. Create subnets
SUBNET1_ID=$(aws ec2 create-subnet \
    --vpc-id $VPC_ID \
    --cidr-block 10.0.1.0/24 \
    --availability-zone ${AWS_REGION}a \
    --query 'Subnet.SubnetId' \
    --output text)

SUBNET2_ID=$(aws ec2 create-subnet \
    --vpc-id $VPC_ID \
    --cidr-block 10.0.2.0/24 \
    --availability-zone ${AWS_REGION}b \
    --query 'Subnet.SubnetId' \
    --output text)

# 11. Create security group
SECURITY_GROUP_ID=$(aws ec2 create-security-group \
    --group-name anypay-websockets-sg \
    --description "Security group for Anypay WebSockets" \
    --vpc-id $VPC_ID \
    --query 'GroupId' \
    --output text)

# 12. Add inbound rule for WebSocket port
aws ec2 authorize-security-group-ingress \
    --group-id $SECURITY_GROUP_ID \
    --protocol tcp \
    --port 8080 \
    --cidr 0.0.0.0/0

# 13. Create ECS service
aws ecs create-service \
    --cluster anypay-websockets-cluster \
    --service-name anypay-websockets-service \
    --task-definition anypay-websockets \
    --desired-count 1 \
    --launch-type FARGATE \
    --network-configuration "awsvpcConfiguration={subnets=[$SUBNET1_ID,$SUBNET2_ID],securityGroups=[$SECURITY_GROUP_ID],assignPublicIp=ENABLED}"
