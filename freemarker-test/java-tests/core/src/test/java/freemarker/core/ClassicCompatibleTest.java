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

package freemarker.core;

import static org.hamcrest.Matchers.*;
import static org.junit.Assert.*;

import java.io.IOException;
import java.util.List;

import org.junit.Test;

import freemarker.ext.beans.BeansWrapper;
import freemarker.template.Configuration;
import freemarker.template.TemplateException;
import freemarker.template.TemplateModel;
import freemarker.test.TemplateTest;

/**
 * Note that we also have {@code classic-compatible.ftl} in the "template test suite", but now I prefer adding new test
 * cases in Java instead.
 */
public class ClassicCompatibleTest extends TemplateTest {
    @Override
    protected Configuration createConfiguration() {
        Configuration conf = new Configuration(Configuration.VERSION_2_3_33);
        conf.setClassicCompatible(true);
        return conf;
    }

    /**
     * FREEMARKER-227
     */
    @Test
    public void testLenientValueComparisonInSeqBuiltIns() throws TemplateException, IOException {
        addToDataModel("seq", List.of(1, "2", 3.0, true, "false"));

        assertOutput("${seq?seq_contains(1)?c}", "true");
        assertOutput("${seq?seq_contains('1')?c}", "true");

        assertOutput("${seq?seq_contains(2)?c}", "true");
        assertOutput("${seq?seq_contains('2')?c}", "true");

        assertOutput("${seq?seq_contains(3)?c}", "true");
        assertOutput("${seq?seq_contains('3')?c}", "true");

        assertOutput("${seq?seq_contains(4)?c}", "false");
        assertOutput("${seq?seq_contains('4')?c}", "false");

        assertOutput("${seq?seq_contains('true')?c}", "true");
        assertOutput("${seq?seq_contains(true)?c}", "true");

        assertOutput("${seq?seq_contains('false')?c}", "true");
        assertOutput("${seq?seq_contains(false)?c}", "false"); // Because false is converted to ""

        // These are actually undesirable/confusing, but I guess it was the closes to 1.7.x behavior, and anyway the
        // classicCompatible mode works like this for a very long time now:
        getConfiguration().setNumberFormat("0.0");
        assertOutput("${seq?seq_contains(2)?c}", "false");
        assertOutput("${seq?seq_contains('2')?c}", "true");
        assertOutput("${seq?seq_contains(3)?c}", "true");
        assertOutput("${seq?seq_contains('3')?c}", "false");
        assertOutput("${seq?seq_contains('3.0')?c}", "true");
    }

    @Test
    public void testMissingValueBuiltIns() throws TemplateException, IOException {
        addToDataModel("nothing", TemplateModel.NOTHING);
        assertOutput("[${missing}] [${missing!'-'}]", "[] [-]");
        assertOutput("[${nothing}] [${nothing!'-'}]", "[] []");
    }

    @Test
    public void testBooleanFormat() throws TemplateException, IOException {
        Configuration conf = getConfiguration();
        assertThat(conf.getClassicCompatibleAsInt(), equalTo(1));
        assertThat(conf.getBooleanFormat(), equalTo("true,false"));
        addToDataModel("beanTrue", new BeansWrapper(Configuration.VERSION_2_3_33).wrap(true));
        addToDataModel("beanFalse", new BeansWrapper(Configuration.VERSION_2_3_33).wrap(false));

        assertOutput("[${true}] [${false}]", "[true] []");
        assertOutput("[${beanTrue}] [${beanFalse}]", "[true] []");
        assertOutput("[${true?c}] [${false?c}]", "[true] [false]");
        assertOutput("[${true?string}] [${false?string}]", "[true] [false]");

        conf.setBooleanFormat("y,n");

        assertOutput("[${true}] [${false}]", "[true] []");
        assertOutput("[${beanTrue}] [${beanFalse}]", "[true] []");
        assertOutput("[${true?c}] [${false?c}]", "[true] [false]");
        assertOutput("[${true?string}] [${false?string}]", "[y] [n]");

        conf.setClassicCompatibleAsInt(2);
        assertOutput("[${true}] [${false}]", "[true] []");
        assertOutput("[${beanTrue}] [${beanFalse}]", "[true] [false]");
        assertOutput("[${true?c}] [${false?c}]", "[true] [false]");
        assertOutput("[${true?string}] [${false?string}]", "[y] [n]");
    }

}
