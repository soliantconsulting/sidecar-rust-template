import { fileURLToPath } from "node:url";
import { CfnOutput, CfnParameter, Duration, Fn, Stack } from "aws-cdk-lib";
import { EndpointType, LambdaIntegration, RestApi } from "aws-cdk-lib/aws-apigateway";
import { Effect, PolicyStatement } from "aws-cdk-lib/aws-iam";
import { ApplicationLogLevel, LoggingFormat } from "aws-cdk-lib/aws-lambda";
import { Queue } from "aws-cdk-lib/aws-sqs";
import { RustFunction } from "cargo-lambda-cdk";
import type { Construct } from "constructs";
import {SqsEventSource} from "aws-cdk-lib/aws-lambda-event-sources";

export class LambdaStack extends Stack {
    public constructor(scope: Construct, id: string) {
        super(scope, id);

        const api = new RestApi(this, "RestApi", {
            restApiName: this.stackName,
            endpointTypes: [EndpointType.REGIONAL],
        });

        new CfnOutput(this, "RestApiEndpoint", {
            value: api.urlForPath(),
        });
        new CfnOutput(this, "RestApiName", {
            value: api.restApiName,
        });
        new CfnOutput(this, "RestApiArn", {
            value: Fn.sub(
                `arn:aws:apigateway:$\{AWS::Region}::/restapis/${api.restApiId}`,
            ),
        });

        const configParameterName = new CfnParameter(this, "ConfigParameterName");
        const configParameterVersion = new CfnParameter(this, "ConfigParameterVersion");

        const queue = new Queue(this, "Queue", {
            visibilityTimeout: Duration.minutes(20),
            retentionPeriod: Duration.days(14),
        });

        new CfnOutput(this, "QueueName", {
            value: queue.queueName,
        });
        new CfnOutput(this, "QueueArn", {
            value: queue.queueArn,
        });

        const webhookFunction = new RustFunction(this, "WebhookFunction", {
            memorySize: 256,
            timeout: Duration.seconds(10),
            allowPublicSubnet: true,
            binaryName: "webhook",
            manifestPath: fileURLToPath(new URL("../Cargo.toml", import.meta.url)),
            runtime: "provided.al2023",
            environment: {
                QUEUE_URL: queue.queueUrl,
                CONFIG_PARAMETER_NAME: configParameterName.valueAsString,
                CONFIG_PARAMETER_VERSION: configParameterVersion.valueAsString,
            },
            loggingFormat: LoggingFormat.JSON,
            applicationLogLevelV2: ApplicationLogLevel.INFO,
        });
        webhookFunction.addToRolePolicy(
            new PolicyStatement({
                effect: Effect.ALLOW,
                actions: ["ssm:GetParameter"],
                resources: [
                    Fn.sub(
                        "arn:aws:ssm:${AWS::Region}:${AWS::AccountId}:parameter${ConfigParameterName}",
                    ),
                ],
            }),
        );
        queue.grantSendMessages(webhookFunction);
        api.root.addMethod("POST", new LambdaIntegration(webhookFunction));

        new CfnOutput(this, "WebhookFunctionName", {
            value: webhookFunction.functionName,
        });
        new CfnOutput(this, "WebhookFunctionArn", {
            value: webhookFunction.functionArn,
        });
        new CfnOutput(this, "WebhookFunctionLogGroupName", {
            value: webhookFunction.logGroup.logGroupName,
        });
        new CfnOutput(this, "WebhookFunctionLogGroupArn", {
            value: webhookFunction.logGroup.logGroupArn,
        });

        const processQueueFunction = new RustFunction(this, "ProcessQueueFunction", {
            memorySize: 256,
            timeout: Duration.minutes(5),
            allowPublicSubnet: true,
            binaryName: "process_queue",
            manifestPath: fileURLToPath(new URL("../Cargo.toml", import.meta.url)),
            runtime: "provided.al2023",
            environment: {
                QUEUE_URL: queue.queueUrl,
                CONFIG_PARAMETER_NAME: configParameterName.valueAsString,
                CONFIG_PARAMETER_VERSION: configParameterVersion.valueAsString,
            },
            loggingFormat: LoggingFormat.JSON,
            applicationLogLevelV2: ApplicationLogLevel.INFO,
        });
        processQueueFunction.addToRolePolicy(
            new PolicyStatement({
                effect: Effect.ALLOW,
                actions: ["ssm:GetParameter"],
                resources: [
                    Fn.sub(
                        "arn:aws:ssm:${AWS::Region}:${AWS::AccountId}:parameter${ConfigParameterName}",
                    ),
                ],
            }),
        );
        queue.grantConsumeMessages(processQueueFunction);
        processQueueFunction.addEventSource(new SqsEventSource(queue, {
            batchSize: 1,
            maxBatchingWindow: Duration.seconds(0),
            reportBatchItemFailures: true,
        }));

        new CfnOutput(this, "ProcessQueueFunctionName", {
            value: processQueueFunction.functionName,
        });
        new CfnOutput(this, "ProcessQueueFunctionArn", {
            value: processQueueFunction.functionArn,
        });
        new CfnOutput(this, "ProcessQueueFunctionLogGroupName", {
            value: processQueueFunction.logGroup.logGroupName,
        });
        new CfnOutput(this, "ProcessQueueFunctionLogGroupArn", {
            value: processQueueFunction.logGroup.logGroupArn,
        });
    }
}
