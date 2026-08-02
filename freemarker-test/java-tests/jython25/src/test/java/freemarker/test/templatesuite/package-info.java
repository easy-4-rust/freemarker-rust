/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

/**
 * JUnit test-suite that processes FreeMarker templates and compare their
 * output to reference files.
 * 
 * <p>To add a test-case, go to
 * {@code src/test/resources/freemarker/test/templatesuite/} and inside that
 * directory:</p>
 * 
 * <ol>
 * <li>Add a template to under {@code templates/} with whatever meaningful file name</li>
 * <li>Add the expected output to {@code references/} with exactly the same file name</li>
 * <li>Add a new {@code testcase} element to {@code testcases.xml}</li>
 * <li>If you want to add items to the data-model or do something else special, modify the {@code setUp()} method in
 * {@code src/test/java/freemarker/test/templatesuite/TemplateTestCase.java}
 * </li>
 * </ol>
 */
package freemarker.test.templatesuite;