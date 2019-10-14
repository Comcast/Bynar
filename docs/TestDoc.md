---
title: Bynar Test Document
---

[]{#_Toc21964897 .anchor}Revision History

  Name             Date         Reason for Change                 Version
  ---------------- ------------ --------------------------------- ---------
  Michelle Zhong   10/8/2019    Outline the Test Document         0.1
  Michelle Zhong   10/9/2019    Outline Test Cases                0.2
  Michelle Zhong   10/14/2019   Flush out the Setup Information   0.3
                                                                  
                                                                  
                                                                  
                                                                  

Table of Contents {#table-of-contents .TOCHeading}
=================

[Revision History 2](#_Toc21964897)

[Setup Information and Prerequisites 5](#_Toc21964898)

[Test Cases 5](#test-cases)

[Test Case 1: Bynar Config 5](#test-case-1-bynar-config)

[Description 5](#description)

[Preconditions For Running the Test Case
5](#preconditions-for-running-the-test-case)

[Scenario 5](#scenario)

[Test Case 2: Ceph Backend 5](#test-case-2-ceph-backend)

[Description 5](#description-1)

[Preconditions For Running the Test Case
5](#preconditions-for-running-the-test-case-1)

[Scenario 5](#scenario-1)

[Test Case 3: Backend Module 6](#test-case-3-backend-module)

[Description 6](#description-2)

[Preconditions For Running the Test Case
6](#preconditions-for-running-the-test-case-2)

[Scenario 6](#scenario-2)

[Test Case 4: Host Information 6](#test-case-4-host-information)

[Description 6](#description-3)

[Preconditions For Running the Test Case
6](#preconditions-for-running-the-test-case-3)

[Scenario 6](#scenario-3)

[Test Case 5: Helpers Library 6](#test-case-5-helpers-library)

[Description 6](#description-4)

[Preconditions For Running the Test Case
6](#preconditions-for-running-the-test-case-4)

[Scenario 6](#scenario-4)

[Test Case 6: Client Interface 7](#test-case-6-client-interface)

[Description 7](#description-5)

[Preconditions For Running the Test Case
7](#preconditions-for-running-the-test-case-5)

[Scenario 7](#scenario-5)

[Test Case 7: Support Ticket Creation
7](#test-case-7-support-ticket-creation)

[Description 7](#description-6)

[Preconditions For Running the Test Case
7](#preconditions-for-running-the-test-case-6)

[Scenario 7](#scenario-6)

[Test Case 8: Disk Manager 7](#test-case-8-disk-manager)

[Description 7](#description-7)

[Preconditions For Running the Test Case
7](#preconditions-for-running-the-test-case-7)

[Scenario 7](#scenario-7)

[Test Case 9: Database Logs 8](#test-case-9-database-logs)

[Description 8](#description-8)

[Preconditions For Running the Test Case
8](#preconditions-for-running-the-test-case-8)

[Scenario 8](#scenario-8)

[Test Case 10: State Machine 8](#test-case-10-state-machine)

[Description 8](#description-9)

[Preconditions For Running the Test Case
8](#preconditions-for-running-the-test-case-9)

[Scenario 8](#scenario-9)

[Test Case 11: Hardware Health Test
8](#test-case-11-hardware-health-test)

[Description 8](#description-10)

[Preconditions For Running the Test Case
8](#preconditions-for-running-the-test-case-10)

[Scenario 9](#scenario-10)

[Test Case 12: Bynar 9](#test-case-12-bynar)

[Description 9](#description-11)

[Preconditions For Running the Test Case
9](#preconditions-for-running-the-test-case-11)

[Scenario 9](#scenario-11)

[]{#_Toc21964898 .anchor}Setup Information and Prerequisites

Travis runs continuous integration on Pull Requests to check if changes
made can be integrated without issue. For manual testing, in many cases,
fault injection in the Linux Kernel will be used to simulate bad disks
and other issues. You will need to rebuild your kernel so that it can
use fault injections. Go Here:
<https://wiki.ubuntu.com/Kernel/BuildYourOwnKernel> to learn how to
rebuild your kernel (for Ubuntu) and add the CONFIG\_FAULT\_INJECTION
flag to your kernel config file.

Test Cases
==========

Test Case 1: Bynar Config
-------------------------

### Description

This case consists of the steps required to test Bynar\'s Config File

### Preconditions For Running the Test Case

You will need to have config files set up for testing the
deserialization as well as the use of some of the information in the
config files. For that, you will need to set up test files to test the
proper parsing and usage of config input. You will need to prepare
working inputs for your test cases.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 2: Ceph Backend
-------------------------

### Description

This case consists of the steps required to test Bynar's Ceph Backend

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description            Execution Steps   Expected Result                                                                Actual Result
  ---- ---------------------- ----------------- ------------------------------------------------------------------------------ ---------------
       Test journal sorting                     Journal devices should be sorted from least to greatest number of partitions   
                                                                                                                               
                                                                                                                               
                                                                                                                               

Test Case 3: Backend Module
---------------------------

### Description

This case consists of the steps required to test Bynar's Backend Module

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 4: Host Information
-----------------------------

### Description

This case consists of the steps required to test Bynar's Host
Information Module

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 5: Helpers Library
----------------------------

### Description

This case consists of the steps required to test Bynar's Helper Library

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 6: Client Interface
-----------------------------

### Description

This case consists of the steps required to test Bynar's Client
Interface

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 7: Support Ticket Creation
------------------------------------

### Description

This case consists of the steps required to test Bynar's Support Ticket
Creation Module

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 8: Disk Manager
-------------------------

### Description

This case consists of the steps required to test Bynar's Disk Manager

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 9: Database Logs
--------------------------

### Description

This case consists of the steps required to test Bynar's Database
Logging Module

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 10: State Machine
---------------------------

### Description

This case consists of the steps required to test Bynar's Disk Test State
Machine

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 11: Hardware Health Test
----------------------------------

### Description

This case consists of the steps required to test Bynar's Hardware Health
Test Module

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
                                                         
                                                         
                                                         
                                                         

Test Case 12: Bynar
-------------------

### Description

This case consists of the steps required to test Bynar's main program

### Preconditions For Running the Test Case

Root permissions are necessary when running these test cases as the test
functions will attempt to create loopback devices, mount them, check
their filesystems, etc. The nightly compiler for Rust is also currently
required for testing as Mocktopus currently uses features that are not
yet in stable Rust.

### Scenario

#### Test Case

  ID   Description   Execution Steps   Expected Result   Actual Result
  ---- ------------- ----------------- ----------------- ---------------
