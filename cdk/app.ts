import { createCdkApp } from "@soliantconsulting/sidecar-deploy-utils";
import { LambdaStack } from "./lambda-stack.js";

const app = createCdkApp(LambdaStack);
app.synth();
